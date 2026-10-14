//! Completion for mnemonics, arguments, imports, flags and dict flag entries.

use std::{
    fs,
    path::{Path, PathBuf},
};

use lsp_types::{
    Command, CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit,
    Documentation, InsertTextFormat, MarkupContent, MarkupKind, Position, TextEdit, Url,
};
use tasmc::core::{
    consts::{ENTRY_POINT, INIT_ROUTINE},
    flags::{FlagValueType, get_flag_type, resolve_flag_int},
    structs::{Hitbox, InstrType, TasmPrimitive, TasmValue},
};

use crate::{
    index::{AliasDef, DocIndex, safe_to_value},
    line::parse_instr_line,
    spec::{
        self, FlagInfo, OP_VALUES, ROUNDSIGN_VALUES, all_instrs, flag_doc, flags_for, instr_doc,
        instr_info, prim_fits, signature,
    },
    text::{byte_of_col, enclosing_routine, is_indented, range, shorten, split_lines, utf16_col},
    world::World,
};

/// Sort groups: literals, then local symbols, then external symbols, then the standard library.
const SORT_LITERAL: &str = "0";
const SORT_LOCAL: &str = "1";
const SORT_EXTERNAL: &str = "2";
const SORT_LAST: &str = "3";

/// The document being completed in.
struct Source<'a> {
    uri: &'a Url,
    lines: &'a [&'a str],
}

/// Unimported symbols are only suggested once this much of a name has been typed.
const AUTO_IMPORT_MIN_CHARS: usize = 2;

struct Ctx<'a> {
    line_idx: usize,
    line: &'a str,
    /// Byte span replaced by the completion (start of the partial token .. cursor).
    start: usize,
    end: usize,
}

impl Ctx<'_> {
    fn partial(&self) -> &str {
        &self.line[self.start..self.end]
    }

    fn item(&self, label: String, kind: CompletionItemKind, sort: &str) -> CompletionItem {
        CompletionItem {
            sort_text: Some(format!("{sort}{label}")),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                range(self.line_idx, self.line, self.start, self.end),
                label.clone(),
            ))),
            label,
            kind: Some(kind),
            ..Default::default()
        }
    }

    fn snippet(&self, label: &str, snippet: &str, description: &str) -> CompletionItem {
        let mut item = self.item(label.into(), CompletionItemKind::SNIPPET, SORT_LITERAL);
        set_text(&mut item, snippet);
        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
        describe(&mut item, description);
        item
    }

    /// External symbols go after everything else unless the user is typing their exact prefix.
    fn external_sort(&self, label: &str) -> &'static str {
        let partial = self.partial();
        if !partial.is_empty() && label.starts_with(partial) {
            SORT_LOCAL
        } else {
            SORT_EXTERNAL
        }
    }
}

fn set_text(item: &mut CompletionItem, text: &str) {
    if let Some(CompletionTextEdit::Edit(edit)) = &mut item.text_edit {
        edit.new_text = text.to_string();
    }
}

fn describe(item: &mut CompletionItem, description: &str) {
    item.label_details = Some(CompletionItemLabelDetails {
        detail: None,
        description: Some(description.to_string()),
    });
}

fn markdown(s: String) -> Documentation {
    Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: s,
    })
}

fn retrigger() -> Command {
    Command::new("".into(), "editor.action.triggerSuggest".into(), None)
}

pub fn complete(world: &World, uri: &Url, text: &str, pos: Position) -> Vec<CompletionItem> {
    let lines = split_lines(text);
    let Some(&line) = lines.get(pos.line as usize) else {
        return vec![];
    };
    let line_idx = pos.line as usize;
    let col = byte_of_col(line, pos.character);
    let before = &line[..col];
    if before.contains(';') {
        return vec![];
    }
    let Some(parts) = parse_instr_line(before) else {
        return vec![];
    };
    let in_init = enclosing_routine(&lines, line_idx).as_deref() == Some(INIT_ROUTINE);
    let mnemonic = parts.mnemonic_text(before);
    let instr = mnemonic.to_ascii_uppercase();
    let index = world.index_for(uri, text);

    if let Some((flags_start, _)) = parts.flags {
        let full = parse_instr_line(line)
            .and_then(|p| p.flags)
            .unwrap_or((flags_start, col));
        return complete_flags(
            world,
            &index,
            &Source { uri, lines: &lines },
            line_idx,
            line,
            &instr,
            flags_start,
            col,
            full,
        );
    }

    let Some(((arg_start, _), _)) = parts.args.last() else {
        let ctx = Ctx {
            line_idx,
            line,
            start: parts.mnemonic.0,
            end: col,
        };
        return complete_mnemonic(&ctx, in_init);
    };
    let ctx = Ctx {
        line_idx,
        line,
        start: *arg_start,
        end: col,
    };
    let arg_idx = parts.args.len() - 1;

    match instr.as_str() {
        "IMPORT" | "IMPORTSTD" if arg_idx == 0 => {
            complete_import(world, uri, &ctx, parts.mnemonic, mnemonic)
        }
        "ALIAS" | "IMPORT" | "IMPORTSTD" | "RAW" | "RAWTRG" => vec![],
        _ => {
            let prior = parts.args[..arg_idx]
                .iter()
                .map(|(_, raw)| world.resolve(&index, raw).map(|r| r.value))
                .collect::<Result<Vec<_>, _>>();
            match prior {
                Ok(prior) => {
                    let src = Source { uri, lines: &lines };
                    complete_arg(world, &index, &src, &ctx, &instr, &prior)
                }
                Err(_) => vec![],
            }
        }
    }
}

fn complete_mnemonic(ctx: &Ctx, in_init: bool) -> Vec<CompletionItem> {
    let upper = ctx.partial().chars().any(|c| c.is_ascii_uppercase());
    all_instrs()
        .into_iter()
        .filter(|info| in_init || !info.init_only)
        .map(|info| {
            let name = if upper {
                info.name.to_string()
            } else {
                info.name.to_ascii_lowercase()
            };
            let has_args = info.argsets.is_empty() || info.argsets.iter().any(|s| !s.is_empty());
            let mut item = ctx.item(name.clone(), CompletionItemKind::KEYWORD, SORT_LOCAL);
            if has_args {
                set_text(&mut item, &format!("{name} "));
                item.command = Some(retrigger());
            }
            item.detail = Some(match info.argsets.as_slice() {
                [] => spec::PSEUDO_INSTRS
                    .iter()
                    .find(|p| p.0 == info.name)
                    .map(|p| p.1.to_string())
                    .unwrap_or_default(),
                [one] => signature(info.name, one),
                [first, rest @ ..] => format!(
                    "{} (+{} overload{})",
                    signature(info.name, first),
                    rest.len(),
                    if rest.len() == 1 { "" } else { "s" }
                ),
            });
            item.documentation = Some(markdown(instr_doc(&info)));
            item
        })
        .collect()
}

fn complete_arg(
    world: &World,
    index: &DocIndex,
    src: &Source,
    ctx: &Ctx,
    instr: &str,
    prior: &[TasmValue],
) -> Vec<CompletionItem> {
    let Some(info) = instr_info(instr) else {
        return vec![];
    };
    let idx = prior.len();
    let allowed: Vec<&TasmPrimitive> = info
        .argsets
        .iter()
        .filter(|sig| sig.len() > idx && prior.iter().zip(sig.iter()).all(|(v, p)| prim_fits(p, v)))
        .map(|sig| &sig[idx])
        .collect();
    if allowed.is_empty() {
        return vec![];
    }
    let accepts = |v: &TasmValue| allowed.iter().any(|p| prim_fits(p, v));
    let mut items = vec![];

    // literals
    if accepts(&TasmValue::Counter(1)) {
        items.push(ctx.snippet("c_", "c${1}", "counter"));
    }
    if accepts(&TasmValue::Timer(1)) {
        items.push(ctx.snippet("t_", "t${1}", "timer"));
    }
    let writes_target = info.itype == Some(InstrType::Arithmetic) && idx == 0;
    for name in ["POINTS", "ATTEMPTS", "MAINTIME"] {
        if writes_target && name != "POINTS" {
            continue; // E0004: these can't be written to
        }
        if let Ok(v) = safe_to_value(name)
            && accepts(&v)
        {
            let mut item = ctx.item(name.into(), CompletionItemKind::CONSTANT, SORT_LITERAL);
            describe(&mut item, "item");
            items.push(item);
        }
    }
    if accepts(&TasmValue::Number(0.5)) {
        items.push(ctx.snippet("_", "${1:0}", "number"));
    } else if accepts(&TasmValue::Number(1.0)) {
        items.push(ctx.snippet("_", "${1:0}", "integer"));
    }
    if accepts(&TasmValue::Group(1)) {
        items.push(ctx.snippet("g_", "g${1}", "group"));
    }
    if accepts(&TasmValue::CollBlock(Hitbox::CollBlock(1))) {
        items.push(ctx.snippet("b_", "b${1}", "collision block"));
    }
    for name in ["COLL_P1", "COLL_P2", "COLL_P_ANY"] {
        if let Ok(v) = safe_to_value(name)
            && accepts(&v)
        {
            let mut item = ctx.item(name.into(), CompletionItemKind::CONSTANT, SORT_LITERAL);
            describe(&mut item, &spec::value_kind(&v));
            items.push(item);
        }
    }

    // local routines and aliases
    if accepts(&TasmValue::Group(1)) {
        for r in index.routines.iter().filter(|r| r.name != INIT_ROUTINE) {
            let mut item = ctx.item(r.name.clone(), CompletionItemKind::FUNCTION, SORT_LOCAL);
            describe(&mut item, "routine");
            items.push(item);
        }
    }
    for a in &index.aliases {
        if let Ok(r) = world.resolve(index, &a.name)
            && accepts(&r.value)
        {
            let mut item = ctx.item(a.name.clone(), CompletionItemKind::CONSTANT, SORT_LOCAL);
            describe(&mut item, &shorten(&a.value, 24));
            item.detail = Some(format!("alias {}, {}", a.name, a.value));
            items.push(item);
        }
    }

    // `module::symbol` from imported modules
    for_each_import(world, index, |module, m| {
        if accepts(&TasmValue::Group(1)) {
            for r in m
                .routines
                .iter()
                .filter(|r| r.name != INIT_ROUTINE && !is_internal(&r.name))
            {
                let label = format!("{module}::{}", r.name);
                let mut item = ctx.item(
                    label.clone(),
                    CompletionItemKind::FUNCTION,
                    ctx.external_sort(&label),
                );
                describe(&mut item, "routine");
                items.push(item);
            }
        }
        for a in m.aliases.iter().filter(|a| !is_internal(&a.name)) {
            if let Ok(v) = safe_to_value(&a.value)
                && accepts(&v)
            {
                let label = format!("{module}::{}", a.name);
                let mut item = ctx.item(
                    label.clone(),
                    CompletionItemKind::CONSTANT,
                    ctx.external_sort(&label),
                );
                describe(&mut item, &shorten(&a.value, 24));
                item.detail = Some(format!("alias {}, {}", a.name, a.value));
                items.push(item);
            }
        }
    });

    items.extend(unimported_symbols(
        world,
        index,
        src,
        ctx,
        accepts(&TasmValue::Group(1)),
        &|a| safe_to_value(&a.value).is_ok_and(|v| accepts(&v)),
    ));
    items
}

/// Whether a symbol of another module is internal (`__name`) and so isn't suggested there.
/// The module's own `__` symbols are still suggested inside it.
fn is_internal(ident: &str) -> bool {
    ident.starts_with("__")
}

/// Calls `f` once per imported module (first import wins for duplicate identifiers).
fn for_each_import(world: &World, index: &DocIndex, mut f: impl FnMut(&str, &DocIndex)) {
    let mut seen = vec![];
    for import in &index.imports {
        if seen.contains(&import.module) {
            continue;
        }
        seen.push(import.module.clone());
        if let Some(m) = import.path.as_ref().and_then(|p| world.module(p)) {
            f(&import.module, &m.index);
        }
    }
}

fn complete_import(
    world: &World,
    uri: &Url,
    ctx: &Ctx,
    mnemonic_span: (usize, usize),
    mnemonic: &str,
) -> Vec<CompletionItem> {
    let is_std = mnemonic.eq_ignore_ascii_case("IMPORTSTD");
    let mut items = vec![];
    let this_file = uri.to_file_path().ok();

    if !is_std && let Some(dir) = this_file.as_ref().and_then(|f| f.parent()) {
        for rel in tasm_files(dir) {
            if this_file.as_deref() == Some(dir.join(&rel).with_extension("tasm").as_path()) {
                continue;
            }
            let mut item = ctx.item(format!("{rel}.tasm"), CompletionItemKind::FILE, SORT_LOCAL);
            set_text(&mut item, &rel);
            item.filter_text = Some(rel.clone());
            items.push(item);
        }
    }

    let upper = mnemonic.chars().any(|c| c.is_ascii_uppercase());
    for rel in tasm_files(&world.stdlib) {
        let label = if is_std {
            format!("{rel}.tasm")
        } else {
            format!("stdlib/{rel}.tasm")
        };
        let mut item = ctx.item(label, CompletionItemKind::MODULE, SORT_LAST);
        set_text(&mut item, &rel);
        item.filter_text = Some(rel.clone());
        if !is_std {
            item.additional_text_edits = Some(vec![TextEdit::new(
                range(ctx.line_idx, ctx.line, mnemonic_span.0, mnemonic_span.1),
                if upper { "IMPORTSTD" } else { "importstd" }.into(),
            )]);
        }
        describe(&mut item, "stdlib");
        items.push(item);
    }
    items
}

/// `.tasm` files under `dir` as `/`-separated paths without extension.
fn tasm_files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, rel: &Path, depth: usize, out: &mut Vec<String>) {
        if depth > 4 || out.len() > 500 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut entries = entries.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            let path = entry.path();
            let rel_path: PathBuf = rel.join(&name);
            if path.is_dir() {
                walk(&path, &rel_path, depth + 1, out);
            } else if path.extension().is_some_and(|e| e == "tasm") {
                let s = rel_path
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(s);
            }
        }
    }
    let mut out = vec![];
    walk(dir, Path::new(""), 0, &mut out);
    out
}

#[allow(clippy::too_many_arguments)]
fn complete_flags(
    world: &World,
    index: &DocIndex,
    src: &Source,
    line_idx: usize,
    line: &str,
    instr: &str,
    flags_start: usize,
    col: usize,
    full_region: (usize, usize),
) -> Vec<CompletionItem> {
    let typed = &line[flags_start..col];

    // inside an unclosed `{`: dict entry
    if let Some(open) = typed.rfind('{')
        && typed.rfind('}').is_none_or(|close| close < open)
    {
        let entry = typed
            .rfind(|c: char| c.is_whitespace() || matches!(c, '{' | ',' | '='))
            .map(|i| i + 1)
            .unwrap_or(0);
        let ctx = Ctx {
            line_idx,
            line,
            start: flags_start + entry,
            end: col,
        };
        return complete_dict_entry(world, index, src, &ctx);
    }

    let token_start = typed.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
    let token = &typed[token_start..];

    if let Some((name, _)) = token.split_once(':') {
        let ctx = Ctx {
            line_idx,
            line,
            start: flags_start + token_start + name.len() + 1,
            end: col,
        };
        let values: &[&str] = match get_flag_type(name) {
            Some(FlagValueType::Bool) => &["true", "false"],
            Some(FlagValueType::Op) => OP_VALUES,
            Some(FlagValueType::RoundSign) => ROUNDSIGN_VALUES,
            _ => &[],
        };
        return values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let mut item = ctx.item(v.to_string(), CompletionItemKind::VALUE, SORT_LITERAL);
                item.sort_text = Some(format!("{i:02}"));
                item
            })
            .collect();
    }

    let used = line[full_region.0..full_region.1]
        .split_whitespace()
        .filter_map(|t| t.split_once(':').map(|(n, _)| n.to_string()))
        .collect::<Vec<_>>();
    let ctx = Ctx {
        line_idx,
        line,
        start: flags_start + token_start,
        end: col,
    };
    flags_for(instr)
        .iter()
        .filter(|f| !used.iter().any(|u| u == f.name))
        .map(|f| flag_item(&ctx, f))
        .collect()
}

fn flag_item(ctx: &Ctx, f: &FlagInfo) -> CompletionItem {
    let name = f.name;
    let (label, text, snippet) = match get_flag_type(name) {
        Some(FlagValueType::Bool) => {
            let value = if f.default.starts_with("true") {
                "false"
            } else {
                "true"
            };
            let s = format!("{name}:{value}");
            (s.clone(), s, false)
        }
        Some(FlagValueType::Dict) => (format!("{name}:{{ }}"), format!("{name}:{{ $1 }}"), true),
        Some(FlagValueType::Op) => (
            format!("{name}:"),
            format!("{name}:${{1|{}|}}", OP_VALUES.join(",")),
            true,
        ),
        Some(FlagValueType::RoundSign) => (
            format!("{name}:"),
            format!("{name}:${{1|{}|}}", ROUNDSIGN_VALUES.join(",")),
            true,
        ),
        _ => (format!("{name}:"), format!("{name}:$1"), true),
    };
    let mut item = ctx.item(label, CompletionItemKind::PROPERTY, SORT_LOCAL);
    set_text(&mut item, &text);
    item.filter_text = Some(name.to_string());
    if snippet {
        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
    }
    describe(&mut item, &format!("default: {}", f.default));
    item.documentation = Some(markdown(flag_doc(f)));
    item
}

fn complete_dict_entry(
    world: &World,
    index: &DocIndex,
    src: &Source,
    ctx: &Ctx,
) -> Vec<CompletionItem> {
    let mut items = vec![];
    for r in index.routines.iter().filter(|r| r.name != INIT_ROUTINE) {
        let mut item = ctx.item(r.name.clone(), CompletionItemKind::FUNCTION, SORT_LOCAL);
        describe(&mut item, "routine");
        items.push(item);
    }
    for a in &index.aliases {
        if world.resolve_dict_entry(index, &a.name).is_ok() {
            let mut item = ctx.item(a.name.clone(), CompletionItemKind::CONSTANT, SORT_LOCAL);
            describe(&mut item, "alias");
            item.detail = Some(format!("alias {}, {}", a.name, a.value));
            items.push(item);
        }
    }
    for_each_import(world, index, |module, m| {
        for r in m
            .routines
            .iter()
            .filter(|r| r.name != INIT_ROUTINE && !is_internal(&r.name))
        {
            let label = format!("{module}::{}", r.name);
            let mut item = ctx.item(
                label.clone(),
                CompletionItemKind::FUNCTION,
                ctx.external_sort(&label),
            );
            describe(&mut item, "external routine");
            items.push(item);
        }
        for a in m.aliases.iter().filter(|a| !is_internal(&a.name)) {
            let label = format!("{module}::{}", a.name);
            if world.resolve_dict_entry(index, &label).is_ok() {
                let mut item = ctx.item(
                    label.clone(),
                    CompletionItemKind::CONSTANT,
                    ctx.external_sort(&label),
                );
                describe(&mut item, "external alias");
                item.detail = Some(format!("alias {}, {}", a.name, a.value));
                items.push(item);
            }
        }
    });
    items.extend(unimported_symbols(world, index, src, ctx, true, &|a| {
        resolve_flag_int(&a.value).is_some()
    }));
    items
}

/// A module that isn't imported yet, but could be: a stdlib module, or a file in the
/// directory of the current file or one of its subdirectories.
struct Candidate {
    module: String,
    /// The line to add to `_init`, e.g. `importstd math`.
    import: String,
    /// Where the module comes from, shown next to the suggestion.
    origin: String,
    index: DocIndex,
}

fn import_candidates(world: &World, uri: &Url, index: &DocIndex) -> Vec<Candidate> {
    let this_file = uri.to_file_path().ok().and_then(|p| p.canonicalize().ok());
    let imported: Vec<PathBuf> = index
        .imports
        .iter()
        .filter_map(|i| i.path.as_ref()?.canonicalize().ok())
        .collect();

    let mut sources: Vec<(PathBuf, String, String)> = tasm_files(&world.stdlib)
        .into_iter()
        .map(|rel| {
            let path = world.stdlib.join(format!("{rel}.tasm"));
            (
                path,
                format!("importstd {rel}"),
                format!("stdlib/{rel}.tasm"),
            )
        })
        .collect();
    if let Some(dir) = uri.to_file_path().ok().as_ref().and_then(|f| f.parent()) {
        sources.extend(tasm_files(dir).into_iter().map(|rel| {
            let path = dir.join(format!("{rel}.tasm"));
            (path, format!("import {rel}"), format!("{rel}.tasm"))
        }));
    }

    let mut out = vec![];
    // the stdlib may also be under this directory; it's then only offered through `importstd`
    let mut seen: Vec<PathBuf> = vec![];
    for (path, import, origin) in sources {
        let Some(m) = world.module(&path) else {
            continue;
        };
        if seen.contains(&m.path) {
            continue;
        }
        seen.push(m.path.clone());
        let module = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_self = this_file.as_ref() == Some(&m.path);
        // importing a file that imports this one would be circular
        let imports_self = m
            .index
            .imports
            .iter()
            .any(|i| i.path.as_ref().and_then(|p| p.canonicalize().ok()) == this_file);
        // programs (files with `_start`) aren't libraries
        if is_self
            || imports_self
            || imported.contains(&m.path)
            || index.import(&module).is_some()
            || m.index.routine(ENTRY_POINT).is_some()
        {
            continue;
        }
        out.push(Candidate {
            module,
            import,
            origin,
            index: m.index.clone(),
        });
    }
    out
}

/// Whether the name part of what was typed (after any `module::`) could complete to `ident`:
/// its characters appear in `ident` in order, ignoring case.
fn could_complete(typed: &str, ident: &str) -> bool {
    let typed = typed
        .rsplit("::")
        .next()
        .unwrap_or(typed)
        .to_ascii_lowercase();
    let mut chars = ident.chars().map(|c| c.to_ascii_lowercase());
    typed.chars().all(|t| chars.any(|c| c == t))
}

/// `module::symbol` suggestions from modules that aren't imported yet. Choosing one also adds
/// the import to `_init`.
fn unimported_symbols(
    world: &World,
    index: &DocIndex,
    src: &Source,
    ctx: &Ctx,
    routines: bool,
    alias_ok: &dyn Fn(&AliasDef) -> bool,
) -> Vec<CompletionItem> {
    let typed = ctx.partial();
    if typed.chars().count() < AUTO_IMPORT_MIN_CHARS {
        return vec![];
    }
    let mut items = vec![];
    for c in import_candidates(world, src.uri, index) {
        let edit = import_edit(src.lines, index, &c.import);
        let mut add = |ident: &str, kind: CompletionItemKind, detail: String| {
            if is_internal(ident) || !could_complete(typed, ident) {
                return;
            }
            let label = format!("{}::{ident}", c.module);
            let mut item = ctx.item(label.clone(), kind, SORT_LAST);
            describe(&mut item, &c.origin);
            item.detail = Some(format!("{detail} (adds `{}`)", c.import));
            item.filter_text = Some(if typed.contains("::") {
                label
            } else {
                ident.to_string()
            });
            item.additional_text_edits = Some(vec![edit.clone()]);
            items.push(item);
        };
        if routines {
            for r in c.index.routines.iter().filter(|r| r.name != INIT_ROUTINE) {
                add(&r.name, CompletionItemKind::FUNCTION, "routine".into());
            }
        }
        for a in c.index.aliases.iter().filter(|a| alias_ok(a)) {
            add(
                &a.name,
                CompletionItemKind::CONSTANT,
                format!("alias {}, {}", a.name, a.value),
            );
        }
    }
    items
}

/// Adds `import` to `_init`: after its last import, or right after `_init:`. Files without
/// `_init` get one at the top.
fn import_edit(lines: &[&str], index: &DocIndex, import: &str) -> TextEdit {
    let Some(init) = index.routine(INIT_ROUTINE) else {
        let start = Position::new(0, 0);
        return TextEdit::new(
            lsp_types::Range::new(start, start),
            format!("{INIT_ROUTINE}:\n    {import}\n\n"),
        );
    };
    let anchor = index
        .imports
        .iter()
        .map(|i| i.line)
        .max()
        .unwrap_or(init.line)
        .max(init.line);
    let indent = lines
        .iter()
        .skip(init.line + 1)
        .find(|l| !l.trim().is_empty() && is_indented(l))
        .map(|l| &l[..l.len() - l.trim_start().len()])
        .unwrap_or("    ");
    let line = lines.get(anchor).copied().unwrap_or_default();
    let end = Position::new(anchor as u32, utf16_col(line, line.len()));
    TextEdit::new(
        lsp_types::Range::new(end, end),
        format!("\n{indent}{import}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Document;

    #[test]
    fn auto_import() {
        let dir = std::env::temp_dir().join(format!("tasm-lsp-ai-{}", std::process::id()));
        let std_dir = dir.join("stdlib");
        fs::create_dir_all(dir.join("lib")).unwrap();
        fs::create_dir_all(&std_dir).unwrap();
        fs::write(
            std_dir.join("math.tasm"),
            "_init:\n    alias PI_ISH, 3\n\nsqrt:\n    nop\n__internal_sq:\n    nop\n",
        )
        .unwrap();
        fs::write(dir.join("lib/utils.tasm"), "square:\n    nop\n").unwrap();
        fs::write(
            dir.join("program.tasm"),
            "_start:\n    nop\nsquash:\n    nop\n",
        )
        .unwrap();
        let main = "_init:\n    alias x, c1\n\nroutine:\n    spawn sq\n";
        let path = dir.join("main.tasm");
        fs::write(&path, main).unwrap();
        let uri = Url::from_file_path(&path).unwrap();
        let mut world = World::new(std_dir, false);
        world
            .docs
            .insert(uri.clone(), Document { text: main.into() });

        let found = |text: &str, pos: Position| {
            complete(&world, &uri, text, pos)
                .into_iter()
                .filter_map(|i| {
                    let edit = i.additional_text_edits.as_ref()?[0].clone();
                    let origin = i.label_details?.description?;
                    Some((i.label, origin, edit.new_text, edit.range.start))
                })
                .collect::<Vec<_>>()
        };
        let s = String::from;
        assert_eq!(
            found(main, Position::new(4, 12)),
            [
                (
                    s("math::sqrt"),
                    s("stdlib/math.tasm"),
                    s("\n    importstd math"),
                    Position::new(0, 6)
                ),
                (
                    s("utils::square"),
                    s("lib/utils.tasm"),
                    s("\n    import lib/utils"),
                    Position::new(0, 6)
                ),
            ]
        );

        // one character isn't enough
        assert!(
            found(
                "_init:\n    nop\nroutine:\n    spawn s\n",
                Position::new(3, 11)
            )
            .is_empty()
        );

        // without `_init`, one is created
        let no_init = found("routine:\n    spawn sq\n", Position::new(1, 12));
        assert_eq!(no_init[0].2, "_init:\n    importstd math\n\n");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn internal_symbols_of_other_modules_are_hidden() {
        let dir = std::env::temp_dir().join(format!("tasm-lsp-int-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("lib.tasm"),
            "_init:\n    alias pub_g, g5\n    alias __priv_g, g6\n\npublic:\n    nop\n__private:\n    nop\n",
        )
        .unwrap();
        let main = "_init:\n    import lib\n    alias __mine, g7\n\n__local:\n    nop\nroutine:\n    spawn \n    spawn public | remap: {\n";
        let path = dir.join("main.tasm");
        fs::write(&path, main).unwrap();
        let uri = Url::from_file_path(&path).unwrap();
        let mut world = World::new(dir.join("stdlib"), false);
        world
            .docs
            .insert(uri.clone(), Document { text: main.into() });

        for pos in [Position::new(7, 10), Position::new(8, 28)] {
            let labels: Vec<String> = complete(&world, &uri, main, pos)
                .into_iter()
                .map(|i| i.label)
                .collect();
            for shown in ["lib::public", "lib::pub_g", "__local", "__mine"] {
                assert!(
                    labels.iter().any(|l| l == shown),
                    "{shown} missing at {pos:?}: {labels:?}"
                );
            }
            for hidden in ["lib::__private", "lib::__priv_g"] {
                assert!(
                    !labels.iter().any(|l| l == hidden),
                    "{hidden} shown at {pos:?}"
                );
            }
        }

        let _ = fs::remove_dir_all(dir);
    }
}
