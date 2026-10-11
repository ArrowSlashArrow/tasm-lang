//! Hover: instruction signatures, argument types/values, flags and imports.
//! Errors are shown by the editor from the diagnostics.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Url};
use tasmc::core::consts::{ENTRY_POINT, INIT_ROUTINE};

use crate::{
    index::DocIndex,
    line::{flag_word, parse_instr_line},
    spec::{find_flag, flag_doc, flags_for, instr_doc, instr_info, value_kind},
    text::{byte_of_col, code_part, display_path, is_indented, range, split_lines, trimmed_span},
    world::{Origin, World},
};

pub fn hover(world: &World, uri: &Url, text: &str, pos: Position) -> Option<Hover> {
    let lines = split_lines(text);
    let line = *lines.get(pos.line as usize)?;
    let line_idx = pos.line as usize;
    let col = byte_of_col(line, pos.character);
    if col > code_part(line).len() {
        return None;
    }
    let index = world.index_for(uri, text);
    let within = |(s, e): (usize, usize)| s <= col && col <= e && s < e;
    let reply = |span: (usize, usize), value: String| {
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(range(line_idx, line, span.0, span.1)),
        })
    };

    if !is_indented(line) {
        let span = trimmed_span(line);
        let name = line[span.0..span.1].strip_suffix(':')?;
        if name.contains(' ') || !within(span) {
            return None;
        }
        let note = match name {
            ENTRY_POINT => "\n\nEntry point of the program.",
            INIT_ROUTINE => {
                "\n\nInitialiser: runs once at the start; holds aliases, imports and init-only instructions."
            }
            _ => "",
        };
        return reply(span, format!("routine `{name}`{note}"));
    }

    let parts = parse_instr_line(line)?;
    let mnemonic = parts.mnemonic_text(line).to_ascii_uppercase();

    if within(parts.mnemonic) {
        let info = instr_info(&mnemonic)?;
        return reply(
            parts.mnemonic,
            format!("**{}**\n\n{}", info.name, instr_doc(&info)),
        );
    }

    for (span, raw) in &parts.args {
        if !within(*span) {
            continue;
        }
        if matches!(mnemonic.as_str(), "IMPORT" | "IMPORTSTD") {
            let import = index.imports.iter().find(|i| i.line == line_idx)?;
            let path = import.path.as_ref()?;
            let kind = if import.is_std {
                "stdlib module"
            } else {
                "module"
            };
            let status = if world.module(path).is_some() {
                ""
            } else {
                " (not found)"
            };
            return reply(
                *span,
                format!(
                    "{kind} `{}`: `{}`{status}

Use its symbols as `{}::symbol`.",
                    import.module,
                    display_path(path),
                    import.module
                ),
            );
        }
        if mnemonic == "ALIAS" {
            return None;
        }
        return reply(*span, describe_arg(world, &index, raw));
    }

    let region = parts.flags?;
    let (span, in_dict) = flag_word(line, region, col)?;
    let word = &line[span.0..span.1];
    if in_dict {
        return reply(span, describe_dict_entry(world, &index, word));
    }
    let name = word.split(':').next().unwrap_or(word);
    match find_flag(&mnemonic, name) {
        Some(f) => reply(span, flag_doc(f)),
        None if flags_for(&mnemonic).is_empty() => {
            reply(span, format!("`{mnemonic}` does not read any flags."))
        }
        None => reply(span, format!("`{name}` is not a flag of `{mnemonic}`.")),
    }
}

fn describe_arg(world: &World, index: &DocIndex, raw: &str) -> String {
    let resolved = match world.resolve(index, raw) {
        Ok(r) => r,
        Err(e) => return error_text(raw, e),
    };
    let kind = value_kind(&resolved.value);
    match resolved.origin {
        Origin::Literal => {
            if kind == "string" && !raw.starts_with('\\') {
                format!(
                    "`{raw}`: string\n\nNot a known alias or routine, so it is read as a string."
                )
            } else {
                format!("`{raw}`: {kind}")
            }
        }
        Origin::LocalAlias(a) => {
            let target = match world.resolve(index, &a.value) {
                Ok(r) => describe_origin(&r.origin, &value_kind(&r.value)),
                Err(_) => kind,
            };
            format!(
                "alias `{}` = `{}`: {target}\n\nDefined on line {}.",
                a.name,
                a.value,
                a.line + 1
            )
        }
        origin => describe_origin(&origin, &kind),
    }
}

fn describe_origin(origin: &Origin, kind: &str) -> String {
    match origin {
        Origin::Literal | Origin::LocalAlias(_) => kind.to_string(),
        Origin::Routine { name, line } => format!("routine `{name}` (line {})", line + 1),
        Origin::ExternalRoutine {
            module,
            ident,
            path,
            line,
        } => format!(
            "routine `{module}::{ident}`\n\n`{}`, line {}",
            display_path(path),
            line + 1
        ),
        Origin::ExternalAlias {
            module,
            alias,
            path,
        } => format!(
            "alias `{module}::{}` = `{}`: {kind}\n\n`{}`, line {}",
            alias.name,
            alias.value,
            display_path(path),
            alias.line + 1
        ),
    }
}

fn describe_dict_entry(world: &World, index: &DocIndex, raw: &str) -> String {
    match world.resolve_dict_entry(index, raw) {
        Ok(_) => match world.resolve(index, raw) {
            Ok(r) => match r.origin {
                Origin::Literal => format!("`{raw}`: {}", value_kind(&r.value)),
                _ => describe_arg(world, index, raw),
            },
            Err(_) => format!("`{raw}`: integer"),
        },
        Err(e) => error_text(raw, e),
    }
}

fn error_text(raw: &str, (code, msg): (i32, String)) -> String {
    if code > 0 {
        format!("`{raw}`\n\n[E{code:0>4}]: {msg}")
    } else {
        format!("`{raw}`\n\n{msg}")
    }
}
