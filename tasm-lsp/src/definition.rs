//! Go to definition (ctrl + click): imported modules, routines and aliases, including
//! `module::symbol` references into other files.

use std::{fs, path::Path};

use lsp_types::{Location, Position, Range, Url};

use crate::{
    index::DocIndex,
    line::{flag_word, parse_instr_line},
    text::{byte_of_col, code_part, display_path, is_indented, range, split_lines, trimmed_span},
    world::{Origin, World},
};

pub fn definition(world: &World, uri: &Url, text: &str, pos: Position) -> Option<Location> {
    let lines = split_lines(text);
    let line = *lines.get(pos.line as usize)?;
    let line_idx = pos.line as usize;
    let col = byte_of_col(line, pos.character);
    if col > code_part(line).len() || !is_indented(line) {
        return None;
    }
    let index = world.index_for(uri, text);
    let within = |(s, e): (usize, usize)| s <= col && col <= e && s < e;

    let parts = parse_instr_line(line)?;
    let mnemonic = parts.mnemonic_text(line).to_ascii_uppercase();
    for (i, (span, raw)) in parts.args.iter().enumerate() {
        if !within(*span) {
            continue;
        }
        return match mnemonic.as_str() {
            "IMPORT" | "IMPORTSTD" => {
                let import = index.imports.iter().find(|i| i.line == line_idx)?;
                let module = world.module(import.path.as_ref()?)?;
                Some(Location::new(file_url(&module.path)?, Range::default()))
            }
            // the alias name itself is the definition; its value may be a routine
            "ALIAS" if i == 0 => None,
            _ => target(world, uri, &lines, &index, raw),
        };
    }

    let (span, in_dict) = flag_word(line, parts.flags?, col)?;
    if !in_dict {
        return None;
    }
    target(world, uri, &lines, &index, &line[span.0..span.1])
}

/// Where the routine or alias that `raw` refers to is defined.
fn target(
    world: &World,
    uri: &Url,
    lines: &[&str],
    index: &DocIndex,
    raw: &str,
) -> Option<Location> {
    match world.resolve(index, raw).ok()?.origin {
        Origin::Literal => None,
        Origin::Routine { line, .. } => {
            Some(Location::new(uri.clone(), routine_range(lines, line)))
        }
        Origin::LocalAlias(a) => Some(Location::new(uri.clone(), alias_range(lines, a.line))),
        Origin::ExternalRoutine { path, line, .. } => {
            let text = file_text(world, &path)?;
            let lines = split_lines(&text);
            Some(Location::new(file_url(&path)?, routine_range(&lines, line)))
        }
        Origin::ExternalAlias { path, alias, .. } => {
            let text = file_text(world, &path)?;
            let lines = split_lines(&text);
            Some(Location::new(
                file_url(&path)?,
                alias_range(&lines, alias.line),
            ))
        }
    }
}

/// The routine name on its label line, without the `:`.
fn routine_range(lines: &[&str], line_idx: usize) -> Range {
    let Some(line) = lines.get(line_idx) else {
        return Range::default();
    };
    let (start, end) = trimmed_span(line);
    let end = if line[start..end].ends_with(':') {
        end - 1
    } else {
        end
    };
    range(line_idx, line, start, end)
}

/// The alias name (first argument) on its `alias` line.
fn alias_range(lines: &[&str], line_idx: usize) -> Range {
    let Some(line) = lines.get(line_idx) else {
        return Range::default();
    };
    match parse_instr_line(line).and_then(|p| p.args.first().map(|a| a.0)) {
        Some((start, end)) => range(line_idx, line, start, end),
        None => range(line_idx, line, 0, 0),
    }
}

/// A module's text, preferring the editor's (possibly unsaved) contents.
fn file_text(world: &World, path: &Path) -> Option<String> {
    let canonical = path.canonicalize().ok()?;
    for (uri, doc) in &world.docs {
        if let Ok(p) = uri.to_file_path()
            && p.canonicalize().ok().as_ref() == Some(&canonical)
        {
            return Some(doc.text.clone());
        }
    }
    fs::read_to_string(&canonical).ok()
}

/// `file://` URL of a path, without the `\\?\` prefix that `canonicalize` adds on Windows.
fn file_url(path: &Path) -> Option<Url> {
    Url::from_file_path(display_path(path)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Document;

    fn world_with(files: &[(&str, &str)]) -> (World, Vec<Url>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("tasm-lsp-def-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut world = World::new(dir.join("stdlib"), false);
        let mut uris = vec![];
        for (name, text) in files {
            let path = dir.join(name);
            fs::write(&path, text).unwrap();
            let uri = Url::from_file_path(&path).unwrap();
            world.docs.insert(
                uri.clone(),
                Document {
                    text: text.to_string(),
                },
            );
            uris.push(uri);
        }
        (world, uris, dir)
    }

    /// Position inside `word` on the first line containing `line_has`.
    fn at(text: &str, line_has: &str, word: &str) -> Position {
        let (l, line) = text
            .lines()
            .enumerate()
            .find(|(_, l)| l.contains(line_has))
            .unwrap();
        let start = line.find(line_has).unwrap() + line_has.find(word).unwrap();
        Position::new(l as u32, start as u32 + 1)
    }

    #[test]
    fn definitions() {
        let lib = "_init:\n    alias lib_value, 7\n\n; helper\nlib_routine:\n    nop\n";
        let main = "_init:\n    import deflib\n    alias number, 123\n\n_start:\n    mov c1, number\n    spawn helper | remap: {c1 = deflib::lib_value}\n    spawn deflib::lib_routine\n\nhelper:\n    nop\n";
        let (world, uris, dir) = world_with(&[("deflib.tasm", lib), ("defmain.tasm", main)]);
        let uri = &uris[1];

        // routine
        let loc = definition(&world, uri, main, at(main, "spawn helper", "helper")).unwrap();
        assert_eq!(loc.uri, *uri);
        assert_eq!(loc.range.start, Position::new(9, 0));
        assert_eq!(loc.range.end, Position::new(9, 6));

        // alias
        let loc = definition(&world, uri, main, at(main, "c1, number", "number")).unwrap();
        assert_eq!(loc.range.start, Position::new(2, 10));

        // import
        let loc = definition(&world, uri, main, at(main, "import deflib", "deflib")).unwrap();
        assert_eq!(loc.uri, uris[0]);

        // external routine and external alias in a dict
        let loc = definition(
            &world,
            uri,
            main,
            at(main, "deflib::lib_routine", "lib_routine"),
        )
        .unwrap();
        assert_eq!(
            (loc.uri.clone(), loc.range.start.line),
            (uris[0].clone(), 4)
        );
        let loc = definition(
            &world,
            uri,
            main,
            at(main, "deflib::lib_value", "lib_value"),
        )
        .unwrap();
        assert_eq!(
            (loc.uri.clone(), loc.range.start.line),
            (uris[0].clone(), 1)
        );

        // literals have no definition
        assert!(definition(&world, uri, main, at(main, "mov c1", "c1")).is_none());

        let _ = fs::remove_dir_all(dir);
    }
}
