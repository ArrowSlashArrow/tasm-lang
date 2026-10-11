//! A lightweight, error-tolerant index of a TASM file: routines, aliases and imports.
//!
//! The compiler's lexer throws its state away when a file has errors, so completion and hover
//! use this index instead. It follows the same rules as `Tasm::index_routines`,
//! `get_aliases_from_init` and `get_imports_from_init` in `rtasm/src/lexer.rs`.

use std::path::{Path, PathBuf};

use tasmc::core::{consts::INIT_ROUTINE, structs::TasmValue};

use crate::text::{code_part, is_indented, trim_span};

#[derive(Debug, Clone)]
pub struct RoutineDef {
    pub name: String,
    pub line: usize,
    /// Doc comment: the unindented comment lines directly above the routine, then the comment
    /// on the label line itself. Each entry is a comment without its `;`.
    pub doc: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AliasDef {
    pub name: String,
    /// The raw (unparsed) value, exactly as the compiler substitutes it.
    pub value: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ImportDef {
    /// The argument as written, e.g. `collections/list`.
    pub raw: String,
    pub is_std: bool,
    pub line: usize,
    /// Module identifier used in `module::symbol` references (file stem of `raw`).
    pub module: String,
    /// Path of the imported `.tasm` file, if it could be determined.
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct DocIndex {
    pub routines: Vec<RoutineDef>,
    pub aliases: Vec<AliasDef>,
    pub imports: Vec<ImportDef>,
    /// Module identifiers used in `module::symbol` references anywhere in the file.
    pub referenced_modules: Vec<String>,
}

impl DocIndex {
    pub fn build(text: &str, file: Option<&Path>, stdlib: &Path) -> Self {
        let mut index = DocIndex::default();
        let mut current: Option<String> = None;
        // comment lines directly above the current line, candidates for a routine's doc
        let mut doc: Vec<String> = vec![];

        for (line_idx, raw_line) in text.lines().enumerate() {
            // same preprocessing as `lexer::parse_file`
            let line = code_part(raw_line).replace('\t', " ");
            let line = line.trim_end();
            if line.trim().is_empty() {
                if raw_line.starts_with(';') {
                    doc.push(comment_text(raw_line));
                } else {
                    doc.clear();
                }
                continue;
            }
            let above = std::mem::take(&mut doc);

            let mut rest = line;
            while let Some(i) = rest.find("::") {
                let module = rest[..i]
                    .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or_default();
                if !module.is_empty() && !index.referenced_modules.iter().any(|m| m == module) {
                    index.referenced_modules.push(module.to_string());
                }
                rest = &rest[i + 2..];
            }

            if !is_indented(line) {
                let trimmed = line.trim();
                current = match trimmed.strip_suffix(':') {
                    Some(name) if !name.contains(' ') => {
                        let mut doc = above;
                        if let Some((_, comment)) = raw_line.split_once(';') {
                            doc.push(comment_text(comment));
                        }
                        // blank comment lines separate paragraphs, but not at the ends
                        while doc.last().is_some_and(String::is_empty) {
                            doc.pop();
                        }
                        let lead = doc.iter().take_while(|l| l.is_empty()).count();
                        doc.drain(..lead);
                        index.routines.push(RoutineDef {
                            name: name.to_string(),
                            line: line_idx,
                            doc,
                        });
                        Some(name.to_string())
                    }
                    _ => None,
                };
                continue;
            }

            if current.as_deref() != Some(INIT_ROUTINE) {
                continue;
            }

            let instr = line.trim();
            let upper = instr.to_ascii_uppercase();
            let args = instr.split('|').next().unwrap_or_default();
            if upper.starts_with("ALIAS ") {
                let parts = args[6..].split(',').map(str::trim).collect::<Vec<_>>();
                if parts.len() == 2
                    && let Ok(TasmValue::String(name)) = safe_to_value(parts[0])
                    && !index.aliases.iter().any(|a| a.name == name)
                {
                    index.aliases.push(AliasDef {
                        name,
                        value: parts[1].to_string(),
                        line: line_idx,
                    });
                }
            } else if upper.starts_with("IMPORT ") || upper.starts_with("IMPORTSTD ") {
                let is_std = upper.starts_with("IMPORTSTD ");
                let raw = args[if is_std { 10 } else { 7 }..].trim().to_string();
                if raw.is_empty() {
                    continue;
                }
                let module = Path::new(&raw)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let base = if is_std {
                    Some(stdlib.to_path_buf())
                } else {
                    file.and_then(|f| f.parent()).map(Path::to_path_buf)
                };
                let path = base.map(|b| {
                    let mut p = b.join(&raw);
                    p.add_extension("tasm");
                    p
                });
                index.imports.push(ImportDef {
                    raw,
                    is_std,
                    line: line_idx,
                    module,
                    path,
                });
            }
        }

        index
    }

    pub fn routine(&self, name: &str) -> Option<&RoutineDef> {
        self.routines.iter().find(|r| r.name == name)
    }

    pub fn alias(&self, name: &str) -> Option<&AliasDef> {
        self.aliases.iter().find(|a| a.name == name)
    }

    pub fn import(&self, module: &str) -> Option<&ImportDef> {
        self.imports.iter().find(|i| i.module == module)
    }
}

/// A comment without its leading `;`s and the space after them.
fn comment_text(comment: &str) -> String {
    let c = comment.trim_start().trim_start_matches(';');
    c.strip_prefix(' ').unwrap_or(c).trim_end().to_string()
}

/// `TasmValue::to_value`, but a panic (e.g. slicing inside a multi-byte character) becomes an
/// error instead of taking the server down.
pub fn safe_to_value(s: &str) -> Result<TasmValue, (i32, String)> {
    let s = s.to_string();
    match std::panic::catch_unwind(move || TasmValue::to_value(&s)) {
        Ok(Ok(v)) => Ok(v),
        Ok(Err((_, msg, code))) => Err((code, msg)),
        Err(_) => Err((-1, "Could not parse value.".into())),
    }
}

/// Splits the argument part of an instruction line into (byte span, trimmed text) pairs.
/// `start`/`end` delimit the argument region inside `line`.
pub fn split_args(line: &str, start: usize, end: usize) -> Vec<((usize, usize), String)> {
    let mut out = vec![];
    let mut seg_start = start;
    for (i, ch) in line[start..end].char_indices() {
        if ch == ',' {
            let span = trim_span(line, seg_start, start + i);
            out.push((span, line[span.0..span.1].to_string()));
            seg_start = start + i + 1;
        }
    }
    let span = trim_span(line, seg_start, end);
    out.push((span, line[span.0..span.1].to_string()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index() {
        let src = "_init:\n\talias n, C1 ; c\n    importstd collections/list\n    import ../lib/helpers\n\n_start:\n    alias bad, 2\n    add n, 1\nloop:\n";
        let idx = DocIndex::build(
            src,
            Some(Path::new("/proj/src/main.tasm")),
            Path::new("/std"),
        );
        assert_eq!(
            idx.routines
                .iter()
                .map(|r| r.name.as_str())
                .collect::<Vec<_>>(),
            ["_init", "_start", "loop"]
        );
        assert_eq!(idx.aliases.len(), 1);
        assert_eq!(idx.aliases[0].value, "C1");
        assert_eq!(idx.imports[0].module, "list");
        assert!(idx.imports[0].is_std);
        assert_eq!(idx.imports[1].module, "helpers");
        assert!(
            idx.imports[1]
                .path
                .as_ref()
                .unwrap()
                .ends_with("helpers.tasm")
        );

        let idx = DocIndex::build(
            "a:
    spawn x::y | remap:{1=lib_2::z}
",
            None,
            Path::new(""),
        );
        assert_eq!(idx.referenced_modules, ["x", "lib_2"]);
    }

    #[test]
    fn docs() {
        let src = "; not this one\n\n; Does a thing.\n;\n; @0tick\nthing: ; inline\n    nop\n    ; not this\nother:\n";
        let idx = DocIndex::build(src, None, Path::new(""));
        assert_eq!(
            idx.routines[0].doc,
            ["Does a thing.", "", "@0tick", "inline"]
        );
        assert!(idx.routines[1].doc.is_empty());
    }

    #[test]
    fn args() {
        let l = "    add c1,  2 ,x";
        let a = split_args(l, 8, l.len());
        assert_eq!(
            a.iter().map(|a| a.1.as_str()).collect::<Vec<_>>(),
            ["c1", "2", "x"]
        );
        assert_eq!(&l[a[1].0.0..a[1].0.1], "2");
    }
}
