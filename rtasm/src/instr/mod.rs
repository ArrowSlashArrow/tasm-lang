use gdlib::gdobj::{
    Item,
    triggers::{CompareOp, Op},
};

use crate::{
    core::{
        HandlerFn,
        flags::FlagValue,
        structs::{HandlerArgs, TasmPrimitive, TasmValue, TasmValueType},
    },
    instr::{fns::*, mem::*},
};

pub mod fns;
pub mod mem;

const GROUP_SPAWN_DELAY: f64 = 0.0044;

// convert a list of type identifiers into a slice
macro_rules! argset {
    (($($arg:ident),*) => $fn:ident) => {
        (&[ $(TasmValueType::Primitive(TasmPrimitive::$arg),)* ], $fn)
    };

    // use this for list args
    ([$argtype:ident] => $fn:ident) => {
        (&[TasmValueType::List(TasmPrimitive::$argtype)], $fn)
    }
}

pub const INSTR_SPEC: &[(
    &str,                             // ident
    bool,                             // exclusive to _init
    &[(&[TasmValueType], HandlerFn)], // handlers
)] = &[
    // inits
    ("MALLOC", true, &[argset!((Int, Int) => malloc)]),
    ("FMALLOC", true, &[argset!((Int, Int) => fmalloc)]),
    ("INITMEM", true, &[argset!([Number] => init_mem)]),
    ("PERS", true, &[argset!((Item) => pers)]),
    ("DISPLAY", true, &[argset!((Item) => display)]),
    ("IOBLOCK", true, &[argset!((Group, Int, String) => ioblock)]),
    // legacy memory
    ("LMALLOC", true, &[argset!((Int) => legacy_malloc)]),
    ("LFMALLOC", true, &[argset!((Int) => legacy_fmalloc)]),
    ("LMFUNC", false, &[argset!(() => legacy_mfunc)]),
    ("LMREAD", false, &[argset!(() => legacy_mread)]),
    ("LMWRITE", false, &[argset!(() => legacy_mwrite)]),
    ("LMPTR", false, &[argset!((Int) => legacy_mptr)]),
    ("LMRESET", false, &[argset!(() => legacy_mreset)]),
    // memory
    (
        "MOV",
        false,
        &[
            argset!((Item, Number) => arithmetic_item_num_mov),
            argset!((Item, Item) => arithmetic_2items_mov),
        ],
    ),
    ("MSET", false, &[argset!(() => mset)]),
    ("MGET", false, &[argset!(() => mget)]),
    // debug
    ("BREAKPOINT", false, &[argset!(() => skip)]),
    // Process
    ("SPAWN", false, &[argset!((Group) => spawn)]),
    // Waits
    ("NOP", false, &[argset!(() => nop)]),
    ("WAIT", false, &[argset!((Int) => wait)]),
    // Arithmetic
    (
        "ADD",
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_add),
            argset!((Item, Number) => arithmetic_item_num_add),
            argset!((Item, Item, Item) => arithmetic_3items_add),
        ],
    ),
    (
        "SUB",
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_sub),
            argset!((Item, Number) => arithmetic_item_num_sub),
            argset!((Item, Item, Item) => arithmetic_3items_sub),
        ],
    ),
    (
        "ADDM",
        false,
        &[
            argset!((Item, Item, Number) => add_mod_2items_num),
            argset!((Item, Item, Item, Number) => add_mod_3items_num),
        ],
    ),
    (
        "SUBM",
        false,
        &[
            argset!((Item, Item, Number) => sub_mod_2items_num),
            argset!((Item, Item, Item, Number) => sub_mod_3items_num),
        ],
    ),
    (
        "ADDD",
        false,
        &[
            argset!((Item, Item, Number) => add_div_2items_num),
            argset!((Item, Item, Item, Number) => add_div_3items_num),
        ],
    ),
    (
        "SUBD",
        false,
        &[
            argset!((Item, Item, Number) => sub_div_2items_num),
            argset!((Item, Item, Item, Number) => sub_div_3items_num),
        ],
    ),
    (
        "MUL",
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_mul),
            argset!((Item, Number) => arithmetic_item_num_mul),
            argset!((Item, Item, Item) => arithmetic_3items_mul),
            argset!((Item, Item, Number) => arithmetic_2items_num_mul),
        ],
    ),
    (
        "DIV",
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_div),
            argset!((Item, Number) => arithmetic_item_num_div),
            argset!((Item, Item, Item) => arithmetic_3items_div),
            argset!((Item, Item, Number) => arithmetic_2items_num_div),
        ],
    ),
    (
        "FLDIV",
        false,
        &[
            argset!((Item, Item) => fldiv_2items),
            argset!((Item, Number) => fldiv_item_num),
            argset!((Item, Item, Item) => fldiv_3items),
            argset!((Item, Item, Number) => fldiv_2items_num),
        ],
    ),
    (
        "SE",
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_eq),
            argset!((Group, Item, Number) => spawn_item_num_eq),
        ],
    ),
    (
        "SNE",
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_ne),
            argset!((Group, Item, Number) => spawn_item_num_ne),
        ],
    ),
    (
        "SL",
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_le),
            argset!((Group, Item, Number) => spawn_item_num_le),
        ],
    ),
    (
        "SLE",
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_leq),
            argset!((Group, Item, Number) => spawn_item_num_leq),
        ],
    ),
    (
        "SG",
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_ge),
            argset!((Group, Item, Number) => spawn_item_num_ge),
        ],
    ),
    (
        "SGE",
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_geq),
            argset!((Group, Item, Number) => spawn_item_num_geq),
        ],
    ),
    (
        "FE",
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_eq),
            argset!((Group, Group, Item, Number) => fork_item_num_eq),
        ],
    ),
    (
        "FNE",
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_ne),
            argset!((Group, Group, Item, Number) => fork_item_num_ne),
        ],
    ),
    (
        "FL",
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_le),
            argset!((Group, Group, Item, Number) => fork_item_num_le),
        ],
    ),
    (
        "FLE",
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_leq),
            argset!((Group, Group, Item, Number) => fork_item_num_leq),
        ],
    ),
    (
        "FG",
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_ge),
            argset!((Group, Group, Item, Number) => fork_item_num_ge),
        ],
    ),
    (
        "FGE",
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_geq),
            argset!((Group, Group, Item, Number) => fork_item_num_geq),
        ],
    ),
    ("SRAND", false, &[argset!((Group, Number) => spawn_random)]),
    (
        "FRAND",
        false,
        &[argset!((Group, Group, Number) => fork_random)],
    ),
    (
        "TSPAWN",
        false,
        &[argset!((Timer, Number, Number, Group) => tspawn)],
    ),
    ("TSTART", false, &[argset!((Timer) => tstart)]),
    ("TSTOP", false, &[argset!((Timer) => tstop)]),
    ("PAUSE", false, &[argset!((Group) => pause)]),
    ("RESUME", false, &[argset!((Group) => resume)]),
    ("STOP", false, &[argset!((Group) => stop)]),
];

// utils

pub fn get_item_spec(item: &TasmValue) -> Option<Item> {
    match item {
        TasmValue::Counter(c) => Some(Item::Counter(*c)),
        TasmValue::Timer(t) => Some(Item::Timer(*t)),
        TasmValue::GDItem(i) => Some(*i),
        _ => None,
    }
}

fn get_flag_value(args: &HandlerArgs, ident: &str, default: FlagValue) -> FlagValue {
    match args.flags.iter().find(|f| f.ident == ident) {
        Some(flag) => flag.value.clone(),
        None => default,
    }
}

fn get_flag_value_opt(args: &HandlerArgs, ident: &str) -> Option<FlagValue> {
    args.flags
        .iter()
        .find(|f| f.ident == ident)
        .map(|f| f.clone().value)
}

fn flag_override<T>(item: &mut T, ident: &str, args: &HandlerArgs)
where
    FlagValue: Into<T>,
{
    if let Some(value) = args.flags.iter().find(|f| f.ident == ident) {
        *item = value.value.clone().into()
    }
}

// Below enums are created for integration with macro.

#[allow(non_camel_case_types)]
enum LowerOp {
    add,
    sub,
    mul,
    div,
    mov,
}
impl LowerOp {
    pub const fn to_op(&self) -> Op {
        match self {
            Self::add => Op::Add,
            Self::sub => Op::Sub,
            Self::mul => Op::Mul,
            Self::div => Op::Div,
            Self::mov => Op::Set,
        }
    }
}

#[allow(non_camel_case_types)]
enum LowerCompOp {
    eq,
    ne,
    le,
    leq,
    ge,
    geq,
}
impl LowerCompOp {
    pub const fn to_op(&self) -> CompareOp {
        match self {
            Self::eq => CompareOp::Equals,
            Self::ne => CompareOp::NotEquals,
            Self::le => CompareOp::Less,
            Self::leq => CompareOp::LessOrEquals,
            Self::ge => CompareOp::Greater,
            Self::geq => CompareOp::GreaterOrEquals,
        }
    }
}
