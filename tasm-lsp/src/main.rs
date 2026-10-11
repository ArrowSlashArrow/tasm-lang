//! Language server for TASM. Speaks LSP over stdio; see `vscode-ext/` for the client.
//!
//! Analysis reuses the compiler (`tasmc`): its lexer produces the diagnostics and its instruction
//! spec and flag registry drive completion and hover.

mod completion;
mod diagnostics;
mod hover;
mod index;
mod line;
mod spec;
mod text;
mod world;

use std::{error::Error, path::PathBuf};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, HoverParams, HoverProviderCapability,
    InitializeParams, PublishDiagnosticsParams, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
        PublishDiagnostics,
    },
    request::{Completion, HoverRequest, Request as _},
};
use serde::Deserialize;

use crate::world::{Document, World};

/// `initializationOptions` sent by the client.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Options {
    /// Overrides the stdlib location used for `IMPORTSTD` (default: the installer's location).
    stdlib_path: Option<String>,
    /// Report E0011 for files without `_start`. Off by default since libraries have no entry point.
    require_entry_point: bool,
}

type AnyError = Box<dyn Error + Sync + Send>;

fn main() -> Result<(), AnyError> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("tasm-lsp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // stdout is the LSP transport, so panics must only ever go to stderr
    std::panic::set_hook(Box::new(|info| eprintln!("tasm-lsp: caught panic: {info}")));

    let (connection, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(
                [" ", ",", "|", ":", "{", "=", "/"]
                    .map(String::from)
                    .to_vec(),
            ),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..Default::default()
    };
    let init = connection.initialize(serde_json::to_value(capabilities)?)?;
    let params: InitializeParams = serde_json::from_value(init)?;
    let options: Options = params
        .initialization_options
        .and_then(|o| serde_json::from_value(o).ok())
        .unwrap_or_default();

    let stdlib = match options.stdlib_path.filter(|s| !s.is_empty()) {
        Some(p) => PathBuf::from(p),
        None => default_stdlib(),
    };
    let mut world = World::new(stdlib, options.require_entry_point);
    main_loop(&connection, &mut world)?;
    // the IO threads only finish once the connection's channels are closed
    drop(connection);
    io_threads.join()?;
    Ok(())
}

fn default_stdlib() -> PathBuf {
    std::panic::catch_unwind(tasmc::lexer::stdlib_dir).unwrap_or_default()
}

fn main_loop(connection: &Connection, world: &mut World) -> Result<(), AnyError> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                let resp = handle_request(world, req);
                connection.sender.send(Message::Response(resp))?;
            }
            Message::Notification(not) => {
                if handle_notification(world, not) {
                    publish_all(connection, world)?;
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn handle_request(world: &World, req: Request) -> Response {
    fn ok<T: serde::Serialize>(id: RequestId, v: T) -> Response {
        Response::new_ok(id, serde_json::to_value(v).unwrap_or_default())
    }
    let id = req.id.clone();
    match req.method.as_str() {
        Completion::METHOD => match serde_json::from_value::<CompletionParams>(req.params) {
            Ok(p) => {
                let pos = p.text_document_position;
                let items = world
                    .docs
                    .get(&pos.text_document.uri)
                    .map(|d| {
                        completion::complete(world, &pos.text_document.uri, &d.text, pos.position)
                    })
                    .unwrap_or_default();
                ok(id, CompletionResponse::Array(items))
            }
            Err(e) => bad_params(id, e),
        },
        HoverRequest::METHOD => match serde_json::from_value::<HoverParams>(req.params) {
            Ok(p) => {
                let pos = p.text_document_position_params;
                let result = world.docs.get(&pos.text_document.uri).and_then(|d| {
                    hover::hover(world, &pos.text_document.uri, &d.text, pos.position)
                });
                ok(id, result)
            }
            Err(e) => bad_params(id, e),
        },
        _ => Response::new_err(
            id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            format!("unhandled method {}", req.method),
        ),
    }
}

fn bad_params(id: RequestId, e: serde_json::Error) -> Response {
    Response::new_err(
        id,
        lsp_server::ErrorCode::InvalidParams as i32,
        e.to_string(),
    )
}

/// Returns whether documents changed (and diagnostics need to be republished).
fn handle_notification(world: &mut World, not: Notification) -> bool {
    match not.method.as_str() {
        DidOpenTextDocument::METHOD => {
            if let Ok(p) = serde_json::from_value::<DidOpenTextDocumentParams>(not.params) {
                world.docs.insert(
                    p.text_document.uri,
                    Document {
                        text: p.text_document.text,
                    },
                );
                return true;
            }
        }
        DidChangeTextDocument::METHOD => {
            if let Ok(p) = serde_json::from_value::<DidChangeTextDocumentParams>(not.params)
                && let Some(change) = p.content_changes.into_iter().last()
            {
                world
                    .docs
                    .insert(p.text_document.uri, Document { text: change.text });
                return true;
            }
        }
        DidCloseTextDocument::METHOD => {
            if let Ok(p) = serde_json::from_value::<DidCloseTextDocumentParams>(not.params) {
                world.docs.remove(&p.text_document.uri);
                world.cleared.push(p.text_document.uri);
                return true;
            }
        }
        _ => {}
    }
    false
}

/// Imports make files depend on each other, so every open document is re-checked on a change.
fn publish_all(connection: &Connection, world: &mut World) -> Result<(), AnyError> {
    let mut batch: Vec<(Url, Vec<lsp_types::Diagnostic>)> =
        world.cleared.drain(..).map(|uri| (uri, vec![])).collect();
    for (uri, doc) in &world.docs {
        batch.push((uri.clone(), diagnostics::diagnostics(world, uri, &doc.text)));
    }
    for (uri, diagnostics) in batch {
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };
        connection
            .sender
            .send(Message::Notification(Notification::new(
                PublishDiagnostics::METHOD.to_string(),
                params,
            )))?;
    }
    Ok(())
}
