//! Hover: instruction signatures, argument types/values, flags and imports.
//! Errors are shown by the editor from the diagnostics.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Url};
use tasmc::{
    core::consts::{ENTRY_POINT, INIT_ROUTINE},
    instr::INSTR_SPEC,
};

use crate::{
    index::DocIndex,
    line::{flag_word, parse_instr_line},
    spec::{flag_doc, get_flag_info, instr_doc, instr_info, value_kind},
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
        let doc = index.routine(name).map(|r| r.doc.as_slice()).unwrap_or(&[]);
        return reply(span, routine_markdown(name, None, doc));
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
    match INSTR_SPEC
        .get(&mnemonic)
        .unwrap()
        .3
        .iter()
        .find(|&&f| f == name)
    {
        Some(f) => reply(span, flag_doc(get_flag_info(f))),
        None if INSTR_SPEC.get(&mnemonic).unwrap().3.is_empty() => {
            reply(span, format!("`{mnemonic}` does not read any flags."))
        }
        None => reply(span, format!("`{name}` is not a flag of `{mnemonic}`.")),
    }
}

/// Hover text for a routine: its name, then its doc comment with the tags rendered.
pub fn routine_markdown(name: &str, module: Option<&str>, doc: &[String]) -> String {
    let mut out = format!("`{name}`");
    if let Some(m) = module {
        out += &format!(" (imported from `{m}`)");
    }

    let mut text: Vec<&str> = vec![];
    let mut params: Vec<String> = vec![];
    let mut returns: Vec<String> = vec![];
    let mut tick: Option<String> = None;
    for line in doc {
        let (tag, rest) = match line.strip_prefix('@') {
            Some(t) => t.split_once(char::is_whitespace).unwrap_or((t, "")),
            None => {
                text.push(line);
                continue;
            }
        };
        let rest = rest.trim();
        match tag.to_ascii_lowercase().as_str() {
            "param" => params.push(format!("* {}", named(rest))),
            "returns" | "return" => returns.push(format!("Returns {}", named(rest))),
            "0tick" => tick = Some("This routine is safe to 0-tick.".into()),
            "no0tick" if rest.is_empty() => tick = Some("This routine cannot be 0-ticked.".into()),
            "no0tick" => tick = Some(format!("This routine cannot be 0-ticked - {rest}")),
            _ => text.push(line),
        }
    }

    let mut sections: Vec<String> = vec![];
    // consecutive lines form a paragraph; blank comment lines separate paragraphs
    let paragraphs = text
        .split(|l| l.is_empty())
        .filter(|p| !p.is_empty())
        .map(|p| p.join("  \n"))
        .collect::<Vec<_>>();
    sections.extend(paragraphs);
    let note = match name {
        ENTRY_POINT => "Entry point of the program.",
        INIT_ROUTINE => {
            "Initialiser: runs once at the start; holds aliases, imports and init-only instructions."
        }
        _ => "",
    };
    if !note.is_empty() {
        sections.push(note.into());
    }
    if !params.is_empty() {
        sections.push(format!("Parameters:\n\n{}", params.join("\n")));
    }
    if !returns.is_empty() {
        sections.push(returns.join("  \n"));
    }
    if let Some(t) = tick {
        sections.push(t);
    }

    if !sections.is_empty() {
        out += "\n\n---\n\n";
        out += &sections.join("\n\n");
    }
    out
}

/// `c1 input counter` -> "`C1`: input counter". Item and group literals are uppercased.
fn named(s: &str) -> String {
    let (name, desc) = s.split_once(char::is_whitespace).unwrap_or((s, ""));
    let is_literal = name.len() > 1
        && matches!(
            name.as_bytes()[0].to_ascii_lowercase(),
            b'c' | b't' | b'g' | b'b'
        )
        && name[1..].bytes().all(|b| b.is_ascii_digit());
    let name = if is_literal {
        name.to_ascii_uppercase()
    } else {
        name.to_string()
    };
    let desc = desc.trim();
    if desc.is_empty() {
        format!("`{name}`")
    } else {
        format!("`{name}`: {desc}")
    }
}

fn describe_arg(world: &World, index: &DocIndex, raw: &str) -> String {
    let resolved = match world.resolve(index, raw) {
        Ok(r) => r,
        Err(e) => return error_text(raw, e),
    };
    let kind = value_kind(&resolved.value);
    match resolved.origin {
        Origin::Routine { name, .. } => {
            let doc = index
                .routine(&name)
                .map(|r| r.doc.clone())
                .unwrap_or_default();
            routine_markdown(&name, None, &doc)
        }
        Origin::ExternalRoutine {
            module,
            ident,
            path,
            ..
        } => {
            let doc = world
                .module(&path)
                .and_then(|m| m.index.routine(&ident).map(|r| r.doc.clone()))
                .unwrap_or_default();
            routine_markdown(&ident, Some(&module), &doc)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routine_docs() {
        let doc = [
            "This routine does something.",
            "@param c1 input counter",
            "@param n",
            "@returns c2 some output",
            "@0tick",
        ]
        .map(String::from);
        assert_eq!(
            routine_markdown("routine", Some("lib"), &doc),
            "`routine` (imported from `lib`)\n\n---\n\nThis routine does something.\n\n\
             Parameters:\n\n* `C1`: input counter\n* `n`\n\nReturns `C2`: some output\n\n\
             This routine is safe to 0-tick."
        );
        let doc = ["@no0tick waits".to_string()];
        assert!(
            routine_markdown("r", None, &doc).ends_with("This routine cannot be 0-ticked - waits")
        );
        assert_eq!(routine_markdown("r", None, &[]), "`r`");
    }
}
