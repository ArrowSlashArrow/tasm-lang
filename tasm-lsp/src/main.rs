use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod analysis;

// todo: generate from INSTR_SPEC
const INSTRUCTIONS: &[(&str, &str)] = &[
    (
        "ADD",
        "`ADD <item> <number|item>` / `ADD <item> <item> <item>`: addition. 1 tick.",
    ),
    ("SUB", "`SUB ...`: subtraction. 1 tick."),
    ("MUL", "`MUL ...`: multiplication. 1 tick."),
    ("DIV", "`DIV ...`: division. 1 tick."),
    ("FLDIV", "`FLDIV ...`: division, result floored. 1 tick."),
    ("MOV", "`MOV <item> <number|item>`: assignment. 1 tick."),
    (
        "SE",
        "`SE <routine> <item> <number|item>`: spawn if equal. 2 ticks.",
    ),
    (
        "SNE",
        "`SNE <routine> <item> <number|item>`: spawn if not equal.",
    ),
    ("SL", "`SL <routine> <item> <number|item>`: spawn if less."),
    (
        "SLE",
        "`SLE <routine> <item> <number|item>`: spawn if less or equal.",
    ),
    (
        "SG",
        "`SG <routine> <item> <number|item>`: spawn if greater.",
    ),
    (
        "SGE",
        "`SGE <routine> <item> <number|item>`: spawn if greater or equal.",
    ),
    (
        "FE",
        "`FE <routine> <routine> <item> <number|item>`: fork if equal.",
    ),
    (
        "SPAWN",
        "`SPAWN <routine>`: spawns a routine without pausing the current one. 1 tick.",
    ),
    (
        "PAUSE",
        "`PAUSE <routine>`: pauses a routine via a stop trigger.",
    ),
    ("RESUME", "`RESUME <routine>`: resumes a paused routine."),
    ("KILL", "`KILL <routine>`: irreversibly stops a routine."),
    (
        "NOP",
        "`NOP`: does nothing for one tick (same as `WAIT 1`).",
    ),
    ("WAIT", "`WAIT <int>`: waits n ticks."),
    (
        "WAITS",
        "`WAITS <float>`: waits `floor(seconds * 240)` ticks.",
    ),
    (
        "ALIAS",
        "`ALIAS <name> <value>`: `_init` only. Defines a global constant alias.",
    ),
    (
        "IMPORT",
        "`IMPORT <path>`: `_init` only. Path is relative to this file, without `.tasm`.",
    ),
    (
        "IMPORTSTD",
        "`IMPORTSTD <path>`: `_init` only. Path is relative to the stdlib directory.",
    ),
    (
        "RAW",
        "`RAW <objects>`: inserts a raw object string into the level.",
    ),
    (
        "RAWTRG",
        "`RAWTRG <objects>`: raw object string treated as part of the routine.",
    ),
];

fn word_at(text: &str, pos: Position) -> Option<(String, u32, u32)> {
    let line = text.lines().nth(pos.line as usize)?;
    let bytes = line.as_bytes();
    let col = (pos.character as usize).min(bytes.len());
    let is_w = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut s = col;
    while s > 0 && is_w(bytes[s - 1]) {
        s -= 1;
    }
    let mut e = col;
    while e < bytes.len() && is_w(bytes[e]) {
        e += 1;
    }
    (s < e).then(|| (line[s..e].to_string(), s as u32, e as u32))
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------
struct Backend {
    client: Client,
    docs: DashMap<Url, String>,
}

impl Backend {
    async fn publish(&self, uri: Url) {
        let Some(text) = self.docs.get(&uri).map(|t| t.clone()) else {
            return;
        };
        let result = analysis::analyze(&text);
        let diags = result
            .diagnostics
            .iter()
            .map(|d| Diagnostic {
                range: Range::new(Position::new(d.line, d.start), Position::new(d.line, d.end)),
                severity: Some(match d.severity {
                    analysis::Severity::Error => DiagnosticSeverity::ERROR,
                    analysis::Severity::Warning => DiagnosticSeverity::WARNING,
                }),
                source: Some("tasm".into()),
                message: d.message.clone(),
                ..Default::default()
            })
            .collect();
        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "tasm-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["|".into(), ":".into()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "tasm-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, p: DidOpenTextDocumentParams) {
        let uri = p.text_document.uri;
        self.docs.insert(uri.clone(), p.text_document.text);
        self.publish(uri).await;
    }

    async fn did_change(&self, mut p: DidChangeTextDocumentParams) {
        // FULL sync: the last change event carries the whole document.
        if let Some(change) = p.content_changes.pop() {
            let uri = p.text_document.uri;
            self.docs.insert(uri.clone(), change.text);
            self.publish(uri).await;
        }
    }

    async fn did_close(&self, p: DidCloseTextDocumentParams) {
        self.docs.remove(&p.text_document.uri);
        self.client
            .publish_diagnostics(p.text_document.uri, vec![], None)
            .await;
    }

    async fn document_symbol(
        &self,
        p: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let Some(text) = self.docs.get(&p.text_document.uri).map(|t| t.clone()) else {
            return Ok(None);
        };
        let a = analysis::analyze(&text);
        #[allow(deprecated)]
        let symbols = a
            .routines
            .iter()
            .map(|r| DocumentSymbol {
                name: r.name.clone(),
                detail: None,
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                range: Range::new(
                    Position::new(r.line, 0),
                    Position::new(r.last_line, r.last_col),
                ),
                selection_range: Range::new(
                    Position::new(r.line, 0),
                    Position::new(r.line, r.name_end),
                ),
                children: None,
            })
            .collect();
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn goto_definition(
        &self,
        p: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = p.text_document_position_params.text_document.uri;
        let pos = p.text_document_position_params.position;
        let Some(text) = self.docs.get(&uri).map(|t| t.clone()) else {
            return Ok(None);
        };
        let Some((word, _, _)) = word_at(&text, pos) else {
            return Ok(None);
        };
        let a = analysis::analyze(&text);
        Ok(a.routines.iter().find(|r| r.name == word).map(|r| {
            GotoDefinitionResponse::Scalar(Location::new(
                uri,
                Range::new(Position::new(r.line, 0), Position::new(r.line, r.name_end)),
            ))
        }))
    }

    async fn hover(&self, p: HoverParams) -> Result<Option<Hover>> {
        let uri = p.text_document_position_params.text_document.uri;
        let pos = p.text_document_position_params.position;
        let Some(text) = self.docs.get(&uri).map(|t| t.clone()) else {
            return Ok(None);
        };
        let Some((word, s, e)) = word_at(&text, pos) else {
            return Ok(None);
        };

        let md = if let Some((_, doc)) = INSTRUCTIONS
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&word))
        {
            Some(doc.to_string())
        } else {
            let a = analysis::analyze(&text);
            a.routines
                .iter()
                .find(|r| r.name == word)
                .map(|r| format!("routine `{}` (line {})", r.name, r.line + 1))
        };
        Ok(md.map(|value| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(Range::new(
                Position::new(pos.line, s),
                Position::new(pos.line, e),
            )),
        }))
    }

    async fn completion(&self, p: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = p.text_document_position.text_document.uri;
        let Some(text) = self.docs.get(&uri).map(|t| t.clone()) else {
            return Ok(None);
        };

        let mut items: Vec<CompletionItem> = INSTRUCTIONS
            .iter()
            .map(|(name, doc)| CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_string(),
                })),
                ..Default::default()
            })
            .collect();
        let a = analysis::analyze(&text);
        items.extend(a.routines.iter().map(|r| CompletionItem {
            label: r.name.clone(),
            kind: Some(CompletionItemKind::FUNCTION),
            ..Default::default()
        }));
        items.extend(a.aliases.iter().map(|n| CompletionItem {
            label: n.clone(),
            kind: Some(CompletionItemKind::CONSTANT),
            ..Default::default()
        }));
        Ok(Some(CompletionResponse::Array(items)))
    }
}

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| Backend {
        client,
        docs: DashMap::new(),
    });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
