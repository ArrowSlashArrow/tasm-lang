use gdlib::gdobj::{
    Item,
    triggers::{CompareOp, Op},
};

use phf::phf_map;

use crate::{
    core::{
        HandlerFn,
        flags::FlagValue,
        structs::{HandlerArgs, InstrType, TasmPrimitive, TasmValue, TasmValueType},
    },
    instr::{fns::*, mem::*},
};

pub mod fns;
pub mod mem;

pub const GROUP_SPAWN_DELAY: f64 = 0.0044;

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

pub type HandlerAssoc = (&'static [TasmValueType], HandlerFn);
pub type Handlers = &'static [HandlerAssoc];
pub const INSTR_SPEC: phf::Map<&'static str, (bool, Handlers, InstrType)> = phf_map! {
    // inits
    // if an instruction can only go in the _init routine, it **MUST** be designated that.
    "MALLOC" => (
        true,
        &[argset!((Int, Int) => malloc)],
        InstrType::Init,
    ),
    "FMALLOC" => (
        true,
        &[argset!((Int, Int) => fmalloc)],
        InstrType::Init,
    ),
    "INITMEM" => (
        true,
        &[argset!([Number] => init_mem)],
        InstrType::Init,
    ),
    "PERS" => (true, &[argset!((Item) => pers)], InstrType::Init),
    "DISPLAY" => (
        true,
        &[argset!((Item) => display)],
        InstrType::Init,
    ),
    "IOBLOCK" => (
        true,
        &[argset!((Group, Int, String) => ioblock)],
        InstrType::Init,
    ),
    // legacy memory
    "LMALLOC" => (
        true,
        &[argset!((Int) => legacy_malloc)],
        InstrType::Init,
    ),
    "LFMALLOC" => (
        true,
        &[argset!((Int) => legacy_fmalloc)],
        InstrType::Init,
    ),
    "LMFUNC" => (
        false,
        &[argset!(() => legacy_mfunc)],
        InstrType::Memory,
    ),
    "LMREAD" => (
        false,
        &[argset!(() => legacy_mread)],
        InstrType::Memory,
    ),
    "LMWRITE" => (
        false,
        &[argset!(() => legacy_mwrite)],
        InstrType::Memory,
    ),
    "LMPTR" => (
        false,
        &[argset!((Int) => legacy_mptr)],
        InstrType::Memory,
    ),
    "LMRESET" => (
        false,
        &[argset!(() => legacy_mreset)],
        InstrType::Memory,
    ),
    // memory
    "MOV" => (
        false,
        &[
            argset!((Item, Number) => arithmetic_item_num_mov),
            argset!((Item, Item) => arithmetic_2items_mov),
        ],
        InstrType::Arithmetic,
    ),
    "MSET" => (false, &[argset!(() => mset)], InstrType::Memory),
    "MGET" => (false, &[argset!(() => mget)], InstrType::Memory),
    // debug
    "BREAKPOINT" => (
        false,
        &[argset!(() => skip)],
        InstrType::Debug,
    ),
    // Process
    "SPAWN" => (
        false,
        &[argset!((Group) => spawn)],
        InstrType::Process,
    ),
    // Waits
    "NOP" => (false, &[argset!(() => nop)], InstrType::Wait),
    "WAIT" => (false, &[argset!((Int) => wait)], InstrType::Wait),
    "WAITS" => (
        false,
        &[argset!((Number) => waits)],
        InstrType::Wait,
    ),
    // Arithmetic
    "ADD" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_add),
            argset!((Item, Number) => arithmetic_item_num_add),
            argset!((Item, Item, Item) => arithmetic_3items_add),
        ],
        InstrType::Arithmetic,
    ),
    "SUB" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_sub),
            argset!((Item, Number) => arithmetic_item_num_sub),
            argset!((Item, Item, Item) => arithmetic_3items_sub),
        ],
        InstrType::Arithmetic,
    ),
    "ADDM" => (
        false,
        &[
            argset!((Item, Item, Number) => add_mod_2items_num),
            argset!((Item, Item, Item, Number) => add_mod_3items_num),
        ],
        InstrType::Arithmetic,
    ),
    "SUBM" => (
        false,
        &[
            argset!((Item, Item, Number) => sub_mod_2items_num),
            argset!((Item, Item, Item, Number) => sub_mod_3items_num),
        ],
        InstrType::Arithmetic,
    ),
    "ADDD" => (
        false,
        &[
            argset!((Item, Item, Number) => add_div_2items_num),
            argset!((Item, Item, Item, Number) => add_div_3items_num),
        ],
        InstrType::Arithmetic,
    ),
    "SUBD" => (
        false,
        &[
            argset!((Item, Item, Number) => sub_div_2items_num),
            argset!((Item, Item, Item, Number) => sub_div_3items_num),
        ],
        InstrType::Arithmetic,
    ),
    "MUL" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_mul),
            argset!((Item, Number) => arithmetic_item_num_mul),
            argset!((Item, Item, Item) => arithmetic_3items_mul),
            argset!((Item, Item, Number) => arithmetic_2items_num_mul),
        ],
        InstrType::Arithmetic,
    ),
    "DIV" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_div),
            argset!((Item, Number) => arithmetic_item_num_div),
            argset!((Item, Item, Item) => arithmetic_3items_div),
            argset!((Item, Item, Number) => arithmetic_2items_num_div),
        ],
        InstrType::Arithmetic,
    ),
    "FLDIV" => (
        false,
        &[
            argset!((Item, Item) => fldiv_2items),
            argset!((Item, Number) => fldiv_item_num),
            argset!((Item, Item, Item) => fldiv_3items),
            argset!((Item, Item, Number) => fldiv_2items_num),
        ],
        InstrType::Arithmetic,
    ),
    "SE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_eq),
            argset!((Group, Item, Number) => spawn_item_num_eq),
        ],
        InstrType::Process,
    ),
    "SNE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_ne),
            argset!((Group, Item, Number) => spawn_item_num_ne),
        ],
        InstrType::Process,
    ),
    "SL" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_le),
            argset!((Group, Item, Number) => spawn_item_num_le),
        ],
        InstrType::Process,
    ),
    "SLE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_leq),
            argset!((Group, Item, Number) => spawn_item_num_leq),
        ],
        InstrType::Process,
    ),
    "SG" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_ge),
            argset!((Group, Item, Number) => spawn_item_num_ge),
        ],
        InstrType::Process,
    ),
    "SGE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_geq),
            argset!((Group, Item, Number) => spawn_item_num_geq),
        ],
        InstrType::Process,
    ),
    "FE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_eq),
            argset!((Group, Group, Item, Number) => fork_item_num_eq),
        ],
        InstrType::Process,
    ),
    "FNE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_ne),
            argset!((Group, Group, Item, Number) => fork_item_num_ne),
        ],
        InstrType::Process,
    ),
    "FL" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_le),
            argset!((Group, Group, Item, Number) => fork_item_num_le),
        ],
        InstrType::Process,
    ),
    "FLE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_leq),
            argset!((Group, Group, Item, Number) => fork_item_num_leq),
        ],
        InstrType::Process,
    ),
    "FG" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_ge),
            argset!((Group, Group, Item, Number) => fork_item_num_ge),
        ],
        InstrType::Process,
    ),
    "FGE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_geq),
            argset!((Group, Group, Item, Number) => fork_item_num_geq),
        ],
        InstrType::Process,
    ),
    "SRAND" => (
        false,
        &[argset!((Group, Number) => spawn_random)],
        InstrType::Process,
    ),
    "FRAND" => (
        false,
        &[argset!((Group, Group, Number) => fork_random)],
        InstrType::Process,
    ),
    "TSPAWN" => (
        false,
        &[argset!((Timer, Number, Number, Group) => tspawn)],
        InstrType::Timer,
    ),
    "TSTART" => (
        false,
        &[argset!((Timer) => tstart)],
        InstrType::Timer,
    ),
    "TSTOP" => (
        false,
        &[argset!((Timer) => tstop)],
        InstrType::Timer,
    ),
    "PAUSE" => (
        false,
        &[argset!((Group) => pause)],
        InstrType::Process,
    ),
    "RESUME" => (
        false,
        &[argset!((Group) => resume)],
        InstrType::Process,
    ),
    "KILL" => (
        false,
        &[argset!((Group) => stop)],
        InstrType::Process,
    ),
    "TOGGLEON" => (
        false,
        &[argset!((Group) => ton)],
        InstrType::Process,
    ),
    "TOGGLEOFF" => (
        false,
        &[argset!((Group) => toff)],
        InstrType::Process,
    ),
    "RAW" => (
        false,
        &[argset!((String) => raw_objs)],
        InstrType::Special,
    ),
    "RAWTRG" => (
        false,
        &[argset!((String) => raw_trigger)],
        InstrType::Special,
    ),
};

// -- utils -- \\

pub fn get_item_spec(item: &TasmValue) -> Option<Item> {
    match item {
        TasmValue::Counter(c) => Some(Item::Counter(*c)),
        TasmValue::Timer(t) => Some(Item::Timer(*t)),
        TasmValue::GDItem(i) => Some(*i),
        _ => None,
    }
}

fn get_flag_value(args: &HandlerArgs, ident: &str, default: FlagValue) -> FlagValue {
    match args.flag_by_ident.get(ident) {
        Some(flag) => flag.value.clone(),
        None => default,
    }
}

fn get_flag_value_opt(args: &HandlerArgs, ident: &str) -> Option<FlagValue> {
    args.flag_by_ident.get(ident).map(|f| f.value.clone())
}

fn flag_override<T>(item: &mut T, ident: &str, args: &HandlerArgs)
where
    FlagValue: Into<T>,
{
    if let Some(flag) = args.flag_by_ident.get(ident) {
        *item = flag.value.clone().into()
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
