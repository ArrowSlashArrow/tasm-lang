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
    consts::INIT_ROUTINE,
    flags::{FlagValueType, get_flag_type},
    structs::{Hitbox, InstrType, TasmPrimitive, TasmValue},
};

use crate::{
    index::{DocIndex, safe_to_value},
    line::parse_instr_line,
    spec::{
        self, FlagInfo, OP_VALUES, ROUNDSIGN_VALUES, all_instrs, flag_doc, flags_for, instr_doc,
        instr_info, prim_fits, signature,
    },
    text::{byte_of_col, enclosing_routine, range, shorten, split_lines},
    world::World,
};

/// Sort groups: literals, then local symbols, then external symbols, then the standard library.
const SORT_LITERAL: &str = "0";
const SORT_LOCAL: &str = "1";
const SORT_EXTERNAL: &str = "2";
const SORT_LAST: &str = "3";

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
                Ok(prior) => complete_arg(world, &index, &ctx, &instr, &prior),
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
            for r in m.routines.iter().filter(|r| r.name != INIT_ROUTINE) {
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
        for a in &m.aliases {
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

    items
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
        return complete_dict_entry(world, index, &ctx);
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

fn complete_dict_entry(world: &World, index: &DocIndex, ctx: &Ctx) -> Vec<CompletionItem> {
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
        for r in m.routines.iter().filter(|r| r.name != INIT_ROUTINE) {
            let label = format!("{module}::{}", r.name);
            let mut item = ctx.item(
                label.clone(),
                CompletionItemKind::FUNCTION,
                ctx.external_sort(&label),
            );
            describe(&mut item, "external routine");
            items.push(item);
        }
        for a in &m.aliases {
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
    items
}
