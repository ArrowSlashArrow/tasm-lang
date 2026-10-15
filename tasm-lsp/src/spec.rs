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

macro_rules! flag {
    ($name:literal, $default:literal, $doc:literal) => {
        FlagInfo {
            name: $name,
            default: $default,
            doc: $doc,
        }
    };
}

const ARITH: &[FlagInfo] = &[
    flag!(
        "itemmod",
        "1, or the number argument",
        "Modifier applied to the operand."
    ),
    flag!(
        "divmod",
        "false (true for `DIV`/`FLDIV` with `Item, Item, Number`)",
        "Divide by the modifier instead of multiplying by it."
    ),
    flag!(
        "op",
        "the instruction's operation",
        "Operation that combines the operands."
    ),
    flag!(
        "iter",
        "the instruction's operation (`=` for three-operand forms)",
        "Operation that writes the result into the target item."
    ),
    flag!(
        "resmode",
        "none (`FLDIV` three-operand forms: floor)",
        "Rounding/sign mode of the combined operands."
    ),
    flag!(
        "finmode",
        "none (`FLDIV` two-operand forms: floor)",
        "Rounding/sign mode of the final result."
    ),
];

const LMOD: FlagInfo = flag!("lmod", "1", "Modifier applied to the left operand.");
const LMODOP: FlagInfo = flag!(
    "lmodop",
    "*",
    "Operation that applies `lmod` to the left operand."
);
const LMODE: FlagInfo = flag!("lmode", "none", "Rounding/sign mode of the left operand.");
const RMOD: FlagInfo = flag!(
    "rmod",
    "1, or the number argument",
    "Modifier applied to the right operand."
);
const RMODOP: FlagInfo = flag!(
    "rmodop",
    "*",
    "Operation that applies `rmod` to the right operand."
);
const RMODE: FlagInfo = flag!("rmode", "none", "Rounding/sign mode of the right operand.");
const ORDERED: FlagInfo = flag!(
    "ordered",
    "true",
    "Spawn the target group's triggers in order."
);
const NOREMAP: FlagInfo = flag!("noremap", "false", "Reset remaps when spawning the group.");
const LNOREMAP: FlagInfo = flag!(
    "lnoremap",
    "false",
    "Reset remaps when spawning the first (true) group."
);
const RNOREMAP: FlagInfo = flag!(
    "rnoremap",
    "false",
    "Reset remaps when spawning the second (false) group."
);

const SPAWN_COMPARE: &[FlagInfo] = &[ORDERED, NOREMAP, LMOD, LMODOP, LMODE, RMOD, RMODOP, RMODE];
const FORK_COMPARE: &[FlagInfo] = &[
    ORDERED, LNOREMAP, RNOREMAP, LMOD, LMODOP, LMODE, RMOD, RMODOP, RMODE,
];
// instant compares don't place spawn triggers, so `ordered`/`noremap` are unused
const INSTANT_COMPARE: &[FlagInfo] = &[LMOD, LMODOP, LMODE, RMOD, RMODOP, RMODE];
const SRAND: &[FlagInfo] = &[ORDERED, NOREMAP];
const FRAND: &[FlagInfo] = &[ORDERED, LNOREMAP, RNOREMAP];
const SPAWN: &[FlagInfo] = &[
    flag!(
        "delay",
        "0.0044 (one tick)",
        "Delay in seconds before the group is spawned."
    ),
    NOREMAP,
    ORDERED,
    flag!(
        "remap",
        "{}",
        "ID remaps for the spawned group: `{from = to, ...}`."
    ),
];
const TSPAWN: &[FlagInfo] = &[
    flag!(
        "tstop",
        "false",
        "Pause the timer when it reaches the stop time."
    ),
    flag!("tmod", "1", "Timer speed modifier."),
    flag!("tpaused", "false", "Start the timer paused."),
    flag!(
        "nover",
        "false",
        "Don't override the timer's current value."
    ),
];
const IOBLOCK: &[FlagInfo] = &[ORDERED];
const EVENT: &[FlagInfo] = &[
    flag!("extra1", "0", "Extra ID of the event trigger."),
    flag!("extra2", "0", "Second extra ID of the event trigger."),
];

/// Flags read by the handlers of `instr` (uppercase).
pub fn flags_for(instr: &str) -> &'static [FlagInfo] {
    match instr {
        "ADD" | "SUB" | "MUL" | "DIV" | "FLDIV" | "MOV" => ARITH,
        "SE" | "SNE" | "SL" | "SLE" | "SG" | "SGE" => SPAWN_COMPARE,
        "FE" | "FNE" | "FL" | "FLE" | "FG" | "FGE" => FORK_COMPARE,
        "ISE" | "ISNE" | "ISL" | "ISLE" | "ISG" | "ISGE" | "IFE" | "IFNE" | "IFL" | "IFLE"
        | "IFG" | "IFGE" => INSTANT_COMPARE,
        "SRAND" => SRAND,
        "FRAND" => FRAND,
        "SPAWN" => SPAWN,
        "TSPAWN" => TSPAWN,
        "IOBLOCK" => IOBLOCK,
        "EVENT" => EVENT,
        _ => &[],
    }
}

pub fn find_flag(instr: &str, name: &str) -> Option<&'static FlagInfo> {
    flags_for(instr).iter().find(|f| f.name == name)
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
    }
}

pub struct InstrInfo {
    pub name: &'static str,
    pub init_only: bool,
    pub itype: Option<InstrType>,
    /// One entry per argset.
    pub argsets: Vec<&'static [TasmPrimitive]>,
}

pub fn instr_info(name: &str) -> Option<InstrInfo> {
    let upper = name.to_ascii_uppercase();
    if let Some((key, (init_only, handlers, itype))) = INSTR_SPEC.get_entry(upper.as_str()) {
        return Some(InstrInfo {
            name: key,
            init_only: *init_only,
            itype: Some(*itype),
            argsets: handlers.iter().map(|(sig, _)| *sig).collect(),
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
    let flags = flags_for(info.name);
    if !flags.is_empty() {
        s.push_str("Flags: ");
        s.push_str(
            &flags
                .iter()
                .map(|f| format!("`{}`", f.name))
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
        for name in INSTR_SPEC.keys() {
            for flag in flags_for(name) {
                assert!(
                    get_flag_type(flag.name).is_some(),
                    "flag {} of {name} is not registered in get_flag_type",
                    flag.name
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
