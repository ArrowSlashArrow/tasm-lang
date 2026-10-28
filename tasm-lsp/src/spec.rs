//! Instruction and flag metadata used for completion and hover.
//!
//! Instruction argsets come straight from `tasmc::instr::INSTR_SPEC`. Which flags each
//! instruction reads (and their defaults) is not recorded in the compiler, so it is listed here;
//! keep it in sync with the `get_flag_value`/`flag_override` calls in `rtasm/src/instr/fns.rs`.

use core::slice;

use gdlib::cclocallevels::gdobj::structs::Item;
use tasmc::{
    core::{
        flags::{FlagValueType, get_flag_type},
        structs::{Hitbox, InstrType, TasmPrimitive, TasmValue, fits_arg_signature},
    },
    instr::INSTR_SPEC,
};

/// Instructions handled by the lexer itself rather than `INSTR_SPEC`. All are `_init`-only.
pub const PSEUDO_INSTRS: &[(&str, &str, &str)] = &[
    (
        "ALIAS",
        "ALIAS ident, value",
        "Names a value. Uses of `ident` are replaced with `value`.",
    ),
    (
        "IMPORT",
        "IMPORT path",
        "Imports a module relative to this file. Its symbols are used as `module::symbol`.",
    ),
    (
        "IMPORTSTD",
        "IMPORTSTD path",
        "Imports a module from the standard library.",
    ),
];

pub struct FlagInfo {
    pub name: &'static str,
    /// Human-readable default. For bool flags this starts with `true`/`false`.
    pub default: &'static str,
    pub doc: &'static str,
}

macro_rules! register_flag {
    ($varname:ident $flag:ident: $default:literal, $doc:literal) => {
        const $varname: FlagInfo = FlagInfo {
            name: stringify!($flag),
            default: $default,
            doc: $doc,
        };
    };
}

register_flag!(FLAG_ITEMMOD itemmod:    "1, or the number argument", "Modifier applied to the operand.");
register_flag!(FLAG_DIVMOD divmod:      "false (true for `DIV`/`FLDIV` with `Item, Item, Number`)","Divide by the modifier instead of multiplying by it.");
register_flag!(FLAG_OP op:              "the instruction's operation","Operation that combines the operands.");
register_flag!(FLAG_ITER iter:          "the instruction's operation (`=` for three-operand forms)","Operation that writes the result into the target item.");
register_flag!(FLAG_RESMODE resmode:    "none (`FLDIV` three-operand forms: floor)","Rounding/sign mode of the combined operands.");
register_flag!(FLAG_FINMODE finmode:    "none (`FLDIV` two-operand forms: floor)","Rounding/sign mode of the final result.");
register_flag!(FLAG_LMOD lmod:          "1",        "Modifier applied to the left operand.");
register_flag!(FLAG_LMODOP lmodop:      "*",        "Operation that applies `lmod` to the left operand.");
register_flag!(FLAG_LMODE lmode:        "none",     "Rounding/sign mode of the left operand.");
register_flag!(FLAG_RMOD rmod:          "1, or the number argument", "Modifier applied to the right operand.");
register_flag!(FLAG_RMODOP rmodop:      "*",        "Operation that applies `rmod` to the right operand.");
register_flag!(FLAG_RMODE rmode:        "none",     "Rounding/sign mode of the right operand.");
register_flag!(FLAG_ORDERED ordered:    "true",     "Spawn the target group's triggers in order.");
register_flag!(FLAG_NOREMAP noremap:    "false",    "Reset remaps when spawning the group.");
register_flag!(FLAG_LNOREMAP lnoremap:  "false",    "Reset remaps when spawning the first (true) group.");
register_flag!(FLAG_RNOREMAP rnoremap:  "false",    "Reset remaps when spawning the second (false) group.");
register_flag!(FLAG_DELAY delay:        "0.0044 (one tick)", "Delay in seconds before the group is spawned.");
register_flag!(FLAG_REMAP remap:        "{}",       "ID remaps for the spawned group: `{from = to, ...}`.");
register_flag!(FLAG_TSTOP tstop:        "false",    "Pause the timer when it reaches the stop time.");
register_flag!(FLAG_TMOD tmod:          "1",        "Timer speed modifier.");
register_flag!(FLAG_TPAUSED tpaused:    "false",    "Start the timer paused.");
register_flag!(FLAG_NOVER nover:        "false",    "Don't override the timer's current value.");
register_flag!(FLAG_EXTRA1 extra1:      "0",        "Extra ID of the event trigger.");
register_flag!(FLAG_EXTRA2 extra2:      "0",        "Second extra ID of the event trigger.");
// sx: ord, nor, lmo, lmp, lmd, rmo, rmp, rmd
// fx: ord, lnr, rnr, lmo, lmp, lmd, rmo, rmp, rmd
// ix: lmo, lmp, lmd, rmo, rmp, rmd
// srand: ord, nor
// frand: ord, lnr, rnr
// spawn: del, nor, ord, rem
// ioblock : ord
// event: ex1, ex2

register_flag!(FLAG_UNKNOWN unknown: "???", "Unknown flag");

/// Returns the FlagInfo for this specific flag
pub fn get_flag_info(flag: &str) -> &'static FlagInfo {
    match flag {
        "itemmod" => &FLAG_ITEMMOD,
        "divmod" => &FLAG_DIVMOD,
        "op" => &FLAG_OP,
        "iter" => &FLAG_ITER,
        "resmode" => &FLAG_RESMODE,
        "finmode" => &FLAG_FINMODE,
        "lmod" => &FLAG_LMOD,
        "lmodop" => &FLAG_LMODOP,
        "lmode" => &FLAG_LMODE,
        "rmod" => &FLAG_RMOD,
        "rmodop" => &FLAG_RMODOP,
        "rmode" => &FLAG_RMODE,
        "ordered" => &FLAG_ORDERED,
        "noremap" => &FLAG_NOREMAP,
        "lnoremap" => &FLAG_LNOREMAP,
        "rnoremap" => &FLAG_RNOREMAP,
        "delay" => &FLAG_DELAY,
        "remap" => &FLAG_REMAP,
        "tstop" => &FLAG_TSTOP,
        "tmod" => &FLAG_TMOD,
        "tpaused" => &FLAG_TPAUSED,
        "nover" => &FLAG_NOVER,
        "extra1" => &FLAG_EXTRA1,
        "extra2" => &FLAG_EXTRA2,
        _ => &FLAG_UNKNOWN,
    }
}

pub fn flag_type_name(t: &FlagValueType) -> &'static str {
    match t {
        FlagValueType::RoundSign => "rounding mode",
        FlagValueType::Float => "number",
        FlagValueType::Op => "operator",
        FlagValueType::Dict => "dict",
        FlagValueType::Bool => "bool",
    }
}

/// Values accepted by rounding/sign flags (see `string_to_roundsign` in core/flags.rs).
pub const ROUNDSIGN_VALUES: &[&str] = &[
    "round", "floor", "ceil", "round+", "round-", "floor+", "floor-", "ceil+", "ceil-", "+", "-",
];
pub const OP_VALUES: &[&str] = &["+", "-", "*", "/"];

pub fn prim_name(p: &TasmPrimitive) -> &'static str {
    match p {
        TasmPrimitive::Item => "Item",
        TasmPrimitive::Timer => "Timer",
        TasmPrimitive::Number => "Number",
        TasmPrimitive::Int => "Int",
        TasmPrimitive::CollBlock => "CollBlock",
        TasmPrimitive::Group => "Group",
        TasmPrimitive::RoutineRef => "RoutineRef",
        TasmPrimitive::ModulePath => "ModulePath",
        TasmPrimitive::String => "String",
    }
}

pub fn prim_fits(p: &TasmPrimitive, v: &TasmValue) -> bool {
    fits_arg_signature(slice::from_ref(v), slice::from_ref(p))
}

/// Short description of a parsed value, e.g. "counter 5" or "number".
pub fn value_kind(v: &TasmValue) -> String {
    match v {
        TasmValue::Counter(c) => format!("counter {c}"),
        TasmValue::Timer(t) => format!("timer {t}"),
        TasmValue::GDItem(i) => match i {
            Item::Attempts => "item (attempts)".into(),
            Item::MainTime => "item (main time)".into(),
            Item::Points => "item (points)".into(),
            _ => "item".into(),
        },
        TasmValue::CollBlock(h) => match h {
            Hitbox::CollBlock(b) => format!("collision block {b}"),
            Hitbox::Player1 => "player 1 hitbox".into(),
            Hitbox::Player2 => "player 2 hitbox".into(),
            Hitbox::EitherPlayer => "either player's hitbox".into(),
        },
        TasmValue::Number(n) if n.fract() == 0.0 => "integer".into(),
        TasmValue::Number(_) => "number".into(),
        TasmValue::Group(g) => format!("group {g}"),
        TasmValue::RoutineRef(_) => "routine".into(),
        TasmValue::ModulePath(_) => "module path".into(),
        TasmValue::String(_) => "string".into(),
        TasmValue::Literal(_) => "string literal".into(),
    }
}

pub struct InstrInfo {
    pub name: &'static str,
    pub init_only: bool,
    pub itype: Option<InstrType>,
    /// One entry per argset.
    pub argsets: Vec<&'static [TasmPrimitive]>,
    pub flags: &'static [&'static str],
}

pub fn instr_info(name: &str) -> Option<InstrInfo> {
    let upper = name.to_ascii_uppercase();
    if let Some((key, (init_only, handlers, itype, flags))) = INSTR_SPEC.get_entry(upper.as_str()) {
        return Some(InstrInfo {
            name: key,
            init_only: *init_only,
            itype: Some(*itype),
            argsets: handlers.iter().map(|(sig, _)| *sig).collect(),
            flags,
        });
    }
    PSEUDO_INSTRS
        .iter()
        .find(|(n, _, _)| *n == upper)
        .map(|(n, _, _)| InstrInfo {
            name: n,
            init_only: true,
            itype: None,
            argsets: vec![],
            flags: &[],
        })
}

pub fn all_instrs() -> Vec<InstrInfo> {
    let mut names: Vec<&str> = INSTR_SPEC.keys().copied().collect();
    names.extend(PSEUDO_INSTRS.iter().map(|p| p.0));
    names.sort();
    names.into_iter().filter_map(instr_info).collect()
}

pub fn signature(name: &str, sig: &[TasmPrimitive]) -> String {
    if sig.is_empty() {
        name.to_string()
    } else {
        format!(
            "{name} {}",
            sig.iter().map(prim_name).collect::<Vec<_>>().join(", ")
        )
    }
}

/// Markdown documentation for an instruction: argsets and flags.
pub fn instr_doc(info: &InstrInfo) -> String {
    let mut s = String::new();
    if let Some((_, sig, doc)) = PSEUDO_INSTRS.iter().find(|p| p.0 == info.name) {
        s.push_str(&format!("```tasm\n{sig}\n```\n{doc}\n\n"));
    } else {
        s.push_str("```tasm\n");
        for sig in &info.argsets {
            s.push_str(&signature(info.name, sig));
            s.push('\n');
        }
        s.push_str("```\n");
        if let Some(t) = info.itype {
            s.push_str(&format!("Type: {t:?}\n\n"));
        }
    }
    if info.init_only {
        s.push_str("Only allowed in `_init`.\n\n");
    }
    let flags = info.flags;
    if !flags.is_empty() {
        s.push_str("Flags: ");
        s.push_str(
            &flags
                .iter()
                .map(|f| format!("`{f}`"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        s.push('\n');
    }
    s
}

pub fn flag_doc(info: &FlagInfo) -> String {
    let t = get_flag_type(info.name)
        .map(|t| flag_type_name(&t))
        .unwrap_or("?");
    format!(
        "**{}**: {t}\n\n{}\n\nDefault: {}",
        info.name, info.doc, info.default
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_tables_match_compiler() {
        for (_, _, _, flags) in INSTR_SPEC.values() {
            for flag in flags.iter() {
                assert!(
                    get_flag_type(flag).is_some(),
                    "flag {flag} is not registered in get_flag_type",
                );
            }
        }
        for name in [
            "ADD", "SUB", "MUL", "DIV", "FLDIV", "MOV", "SE", "FE", "ISE", "IFE", "SRAND", "FRAND",
            "SPAWN", "TSPAWN", "IOBLOCK", "EVENT",
        ] {
            assert!(
                INSTR_SPEC.contains_key(name),
                "{name} is not an instruction"
            );
        }
    }

    #[test]
    fn info() {
        let add = instr_info("add").unwrap();
        assert_eq!(add.name, "ADD");
        assert!(add.argsets.len() >= 3);
        assert!(instr_info("importstd").unwrap().init_only);
        assert!(instr_info("nope").is_none());
    }
}
