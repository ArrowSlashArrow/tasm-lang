//! Diagnostics: the compiler's own lexer errors, plus the checks that the linker and the
//! object-placement stage would do for this file.

use std::{
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind},
};

use gdlib::cclocallevels::gdobj::{meta::GDObjConfig, structs::ItemType};
use lsp_types::{Diagnostic, DiagnosticSeverity, Url};
use tasmc::{
    core::{
        flags::{FlagValue, UnparsedDictFlagEntry},
        structs::{HandlerArgs, InstrType, Instruction, TasmValue, fits_arg_signature},
    },
    instr::{INSTR_SPEC, get_item_spec, placeholder_panic_fn},
};

use crate::{
    index::DocIndex,
    spec::prim_name,
    text::{display_path, range, split_lines, trimmed_span},
    world::{CodedError, External, World, external_error, lex},
};

struct Sink<'a> {
    file: String,
    lines: Vec<&'a str>,
    out: Vec<Diagnostic>,
}

impl Sink<'_> {
    fn push(&mut self, line: usize, code: i32, details: &str) {
        let text = self.lines.get(line).copied().unwrap_or_default();
        let (start, end) = trimmed_span(text);
        let message = if code > 0 {
            format!("{}:{}\n[E{code:0>4}]: {details}", self.file, line + 1)
        } else {
            format!("{}:{}\n{details}", self.file, line + 1)
        };
        let diag = Diagnostic {
            range: range(line, text, start, end),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("tasm".into()),
            message,
            ..Default::default()
        };
        if !self.out.contains(&diag) {
            self.out.push(diag);
        }
    }
}

pub fn diagnostics(world: &World, uri: &Url, text: &str) -> Vec<Diagnostic> {
    let path = uri.to_file_path().ok();
    let file = path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| uri.to_string());
    let fname = path
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.clone());
    let mut sink = Sink {
        file,
        lines: split_lines(text),
        out: vec![],
    };

    let index = world.index_for(uri, text);
    let tasm = match lex(text, &fname, world.require_entry_point) {
        Ok(t) => t,
        Err(e) => {
            sink.push(0, e.errcode, &e.details);
            return sink.out;
        }
    };
    for e in &tasm.errors {
        sink.push(e.line, e.errcode, &e.details);
    }

    for import in &index.imports {
        let module = import.path.as_ref().and_then(|p| world.module(p));
        match module {
            None => sink.push(
                import.line,
                43,
                &format!(
                    "Unable to resolve path for module {} ({}).",
                    import.raw,
                    import
                        .path
                        .as_ref()
                        .map(|p| display_path(p))
                        .unwrap_or_else(|| "file is not saved".into())
                ),
            ),
            Some(m) if !m.errors.is_empty() => {
                let first = &m.errors[0];
                sink.push(
                    import.line,
                    44,
                    &format!(
                        "Unable to compile {} ({} error(s)). First error, line {}: [E{:0>4}] {}",
                        display_path(&m.path),
                        m.errors.len(),
                        first.line + 1,
                        first.errcode,
                        first.details
                    ),
                );
            }
            Some(_) => {
                if let Some(chain) = path
                    .as_deref()
                    .and_then(|p| world.import_cycle(p, &index, &import.module))
                {
                    sink.push(
                        import.line,
                        45,
                        &format!(
                            "Circular dependency between modules: {}",
                            chain.join(" -> ")
                        ),
                    );
                }
            }
        }
    }

    for routine in &tasm.routines {
        for instr in &routine.instructions {
            if let Err((code, msg)) = check_instruction(world, &index, instr) {
                sink.push(instr.line_number, code, &msg);
            }
        }
    }

    sink.out
}

/// Links an instruction's external references and runs its handler, mirroring
/// `linker::post_link_processing` and `Tasm::handle_instruction`.
fn check_instruction(
    world: &World,
    index: &DocIndex,
    instr: &Instruction,
) -> Result<(), CodedError> {
    let mut args = instr.args.clone();
    for arg in args.iter_mut() {
        if let TasmValue::RoutineRef(symbol) = arg {
            *arg = match &symbol.root {
                None => TasmValue::Group(symbol.get_group().unwrap_or(1)),
                Some(module) => match world.lookup(index, module, &symbol.ident) {
                    External::Routine { .. } => TasmValue::Group(1),
                    External::Alias { alias, .. } => crate::index::safe_to_value(&alias.value)?,
                    other => return Err(external_error(other, module, &symbol.ident)),
                },
            };
        }
    }

    let mut handler = instr.handler_fn;
    if handler as usize == placeholder_panic_fn as *const () as usize {
        let (_, handlers, _, _) = INSTR_SPEC
            .get(instr.ident.as_str())
            .ok_or((25, format!("Unrecognized instruction {}", instr.ident)))?;
        handler = match handlers
            .iter()
            .find(|(sig, _)| fits_arg_signature(&args, sig))
        {
            Some((_, h)) => *h,
            None => {
                let types = args
                    .iter()
                    .map(|a| prim_name(&a.get_type()))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err((
                    42,
                    format!(
                        "Instruction {} has no argument handler for the argset [{types}]",
                        instr.ident
                    ),
                ));
            }
        };
    }

    let mut flags = instr.flags.clone();
    for flag in flags.iter_mut() {
        if let FlagValue::ExternRefsDict(entries) = &flag.value {
            let mut dict = vec![];
            for (k, v) in entries {
                dict.push((dict_entry(world, index, k)?, dict_entry(world, index, v)?));
            }
            flag.value = FlagValue::Dict(dict);
        }
    }

    if instr.itype == InstrType::Arithmetic
        && let Some(item) = args.first().and_then(get_item_spec)
        && matches!(item.get_type(), ItemType::Attempts | ItemType::MainTime)
    {
        return Err((
            4,
            format!("Cannot overwrite value of {:?}.", item.get_type()),
        ));
    }

    let flag_by_ident = flags
        .iter()
        .map(|f| (f.ident.clone(), f))
        .collect::<HashMap<_, _>>();
    let handler_args = HandlerArgs {
        args: &args,
        cfg: GDObjConfig::default(),
        curr_group: 1,
        displayed_items: 0,
        flag_by_ident,
        unordered_start: false,
        line: instr.line_number,
        is_release: false,
    };
    match catch_unwind(AssertUnwindSafe(|| handler(handler_args))) {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err((e.errcode, e.details)),
        // handlers unwrap flag values of the wrong type, e.g. `delay:{...}`
        Err(_) => Err((
            -1,
            "The compiler crashed while building this instruction.".into(),
        )),
    }
}

fn dict_entry(
    world: &World,
    index: &DocIndex,
    entry: &UnparsedDictFlagEntry,
) -> Result<i16, CodedError> {
    match entry {
        UnparsedDictFlagEntry::Int(i) => Ok(*i),
        UnparsedDictFlagEntry::Path(p) => match &p.root {
            None => Ok(p.get_group().unwrap_or(1)),
            Some(module) => world.resolve_dict_entry(index, &format!("{module}::{}", p.ident)),
        },
    }
}
