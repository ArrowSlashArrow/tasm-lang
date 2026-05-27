use gdlib::gdobj::{
    Item,
    triggers::{CompareOp, Op},
};

use phf::phf_map;

use crate::{
    core::{
        HandlerFn,
        flags::FlagValue,
        structs::{
            HandlerArgs, InstrIdent, InstrType, Instruction, TasmPrimitive, TasmValue,
            TasmValueType,
        },
    },
    debugger::Emulator,
    instr::{fns::*, mem::*},
};

pub mod fns;
pub mod mem;

pub const GROUP_SPAWN_DELAY: f64 = 0.0044;

// convert a list of type identifiers into a slice
macro_rules! argset {
    // **TEMP FIX**
    // TODO: REMOVE WHEN WE HAVE HANDLERS FOR ALL INSTRUCTIONS!!!!
    (($($arg:ident),*) => $fn:ident) => {
        (&[ $(TasmValueType::Primitive(TasmPrimitive::$arg),)* ], $fn, Emulator::not_implemented)
    };

    (($($arg:ident),*) => $fn:ident, $emu_fn:ident) => {
        (&[ $(TasmValueType::Primitive(TasmPrimitive::$arg),)* ], $fn, Emulator::$emu_fn)
    };

    // THIS IS ALSO A TEMP FIX!!
    ([$argtype:ident] => $fn:ident) => {
        (&[TasmValueType::List(TasmPrimitive::$argtype)], $fn, Emulator::not_implemented)
    };

    // use this for list args
    ([$argtype:ident] => $fn:ident, $emu_fn:ident) => {
        (&[TasmValueType::List(TasmPrimitive::$argtype)], $fn, Emulator::$emu_fn)
    }
}

pub type EmulatorArgs<'a> = &'a Instruction;
pub type EmulatorHandler = fn(&mut Emulator, EmulatorArgs) -> ();
pub type HandlerAssoc = (&'static [TasmValueType], HandlerFn, EmulatorHandler);
pub type Handlers = &'static [HandlerAssoc];
pub const INSTR_SPEC: phf::Map<&'static str, (bool, Handlers, InstrType, InstrIdent)> = phf_map! {
    // inits
    // if an instruction can only go in the _init routine, it **MUST** be designated that.
    "MALLOC" => (
        true,
        &[argset!((Int, Int) => malloc, unreachable)],
        InstrType::Init,
        InstrIdent::MALLOC,
    ),
    "FMALLOC" => (
        true,
        &[argset!((Int, Int) => fmalloc, unreachable)],
        InstrType::Init,
        InstrIdent::FMALLOC,
    ),
    "INITMEM" => (
        true,
        &[argset!([Number] => init_mem, not_implemented)],
        InstrType::Init,
        InstrIdent::INITMEM,
    ),
    "PERS" => (true, &[argset!((Item) => pers, not_implemented)], InstrType::Init, InstrIdent::PERS),
    "DISPLAY" => (
        true,
        &[argset!((Item) => display, unreachable)],
        InstrType::Init,
        InstrIdent::DISPLAY,
    ),
    "IOBLOCK" => (
        true,
        &[argset!((Group, Int, String) => ioblock, unreachable)],
        InstrType::Init,
        InstrIdent::IOBLOCK,
    ),
    // legacy memory
    "LMALLOC" => (
        true,
        &[argset!((Int) => legacy_malloc)],
        InstrType::Init,
        InstrIdent::LMALLOC,
    ),
    "LFMALLOC" => (
        true,
        &[argset!((Int) => legacy_fmalloc)],
        InstrType::Init,
        InstrIdent::LFMALLOC,
    ),
    "LMFUNC" => (
        false,
        &[argset!(() => legacy_mfunc)],
        InstrType::Memory,
        InstrIdent::LMFUNC,
    ),
    "LMREAD" => (
        false,
        &[argset!(() => legacy_mread)],
        InstrType::Memory,
        InstrIdent::LMREAD,
    ),
    "LMWRITE" => (
        false,
        &[argset!(() => legacy_mwrite)],
        InstrType::Memory,
        InstrIdent::LMWRITE,
    ),
    "LMPTR" => (
        false,
        &[argset!((Int) => legacy_mptr)],
        InstrType::Memory,
        InstrIdent::LMPTR,
    ),
    "LMRESET" => (
        false,
        &[argset!(() => legacy_mreset)],
        InstrType::Memory,
        InstrIdent::LMRESET,
    ),
    // memory
    "MOV" => (
        false,
        &[
            argset!((Item, Number) => arithmetic_item_num_mov, arithmetic_2items_mov),
            argset!((Item, Item) => arithmetic_2items_mov, arithmetic_2items_mov),
        ],
        InstrType::Arithmetic,
        InstrIdent::MOV
    ),
    "MSET" => (false, &[argset!(() => mset)], InstrType::Memory, InstrIdent::MSET),
    "MGET" => (false, &[argset!(() => mget)], InstrType::Memory, InstrIdent::MGET),
    // debug
    "BREAKPOINT" => (
        false,
        &[argset!(() => skip, breakpoint)],
        InstrType::Debug,
        InstrIdent::BREAKPOINT,
    ),
    // Process
    "SPAWN" => (
        false,
        &[argset!((Group) => spawn, spawn)],
        InstrType::Process,
        InstrIdent::SPAWN,
    ),
    // Waits
    "NOP" => (false, &[argset!(() => nop, silent_skip)], InstrType::Wait, InstrIdent::NOP),
    "WAIT" => (false, &[argset!((Int) => wait, silent_skip)], InstrType::Wait, InstrIdent::WAIT),
    "WAITS" => (
        false,
        &[argset!((Number) => waits, silent_skip)],
        InstrType::Wait,
        InstrIdent::WAITS,
    ),
    // Arithmetic
    "ADD" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_add, arithmetic_2items_add),
            argset!((Item, Number) => arithmetic_item_num_add, arithmetic_2items_add),
            argset!((Item, Item, Item) => arithmetic_3items_add, arithmetic_3items_add),
        ],
        InstrType::Arithmetic,
        InstrIdent::ADD
    ),
    "SUB" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_sub, arithmetic_2items_sub),
            argset!((Item, Number) => arithmetic_item_num_sub, arithmetic_2items_sub),
            argset!((Item, Item, Item) => arithmetic_3items_sub, arithmetic_3items_sub),
        ],
        InstrType::Arithmetic,
        InstrIdent::SUB
    ),
    "ADDM" => (
        false,
        &[
            argset!((Item, Item, Number) => add_mod_2items_num, arithmetic_3items_addm),
            argset!((Item, Item, Item, Number) => add_mod_3items_num, arithmetic_4items_addm),
        ],
        InstrType::Arithmetic,
        InstrIdent::ADDM
    ),
    "SUBM" => (
        false,
        &[
            argset!((Item, Item, Number) => sub_mod_2items_num, arithmetic_3items_subm),
            argset!((Item, Item, Item, Number) => sub_mod_3items_num, arithmetic_4items_subm),
        ],
        InstrType::Arithmetic,
        InstrIdent::SUBM
    ),
    "ADDD" => (
        false,
        &[
            argset!((Item, Item, Number) => add_div_2items_num, arithmetic_3items_addd),
            argset!((Item, Item, Item, Number) => add_div_3items_num, arithmetic_4items_addd),
        ],
        InstrType::Arithmetic,
        InstrIdent::ADDD
    ),
    "SUBD" => (
        false,
        &[
            argset!((Item, Item, Number) => sub_div_2items_num, arithmetic_3items_subd),
            argset!((Item, Item, Item, Number) => sub_div_3items_num, arithmetic_4items_subd),
        ],
        InstrType::Arithmetic,
        InstrIdent::SUBD
    ),
    "MUL" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_mul, arithmetic_2items_mul),
            argset!((Item, Number) => arithmetic_item_num_mul, arithmetic_2items_mul),
            argset!((Item, Item, Item) => arithmetic_3items_mul, arithmetic_3items_mul),
            argset!((Item, Item, Number) => arithmetic_2items_num_mul, arithmetic_3items_mul),
        ],
        InstrType::Arithmetic,
        InstrIdent::MUL
    ),
    "DIV" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_div, arithmetic_2items_div),
            argset!((Item, Number) => arithmetic_item_num_div, arithmetic_2items_div),
            argset!((Item, Item, Item) => arithmetic_3items_div, arithmetic_3items_div),
            argset!((Item, Item, Number) => arithmetic_2items_num_div, arithmetic_3items_div),
        ],
        InstrType::Arithmetic,
        InstrIdent::DIV
    ),
    "FLDIV" => (
        false,
        &[
            argset!((Item, Item) => fldiv_2items, arithmetic_2items_fldiv),
            argset!((Item, Number) => fldiv_item_num, arithmetic_2items_fldiv),
            argset!((Item, Item, Item) => fldiv_3items, arithmetic_3items_fldiv),
            argset!((Item, Item, Number) => fldiv_2items_num, arithmetic_3items_fldiv),
        ],
        InstrType::Arithmetic,
        InstrIdent::FLDIV
    ),
    "SE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_eq, compare_spawn_eq),
            argset!((Group, Item, Number) => spawn_item_num_eq, compare_spawn_eq),
        ],
        InstrType::Process,
        InstrIdent::SE
    ),
    "SNE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_ne, compare_spawn_ne),
            argset!((Group, Item, Number) => spawn_item_num_ne, compare_spawn_ne),
        ],
        InstrType::Process,
        InstrIdent::SNE
    ),
    "SL" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_le, compare_spawn_lt),
            argset!((Group, Item, Number) => spawn_item_num_le, compare_spawn_lt),
        ],
        InstrType::Process,
        InstrIdent::SL
    ),
    "SLE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_leq, compare_spawn_le),
            argset!((Group, Item, Number) => spawn_item_num_leq, compare_spawn_le),
        ],
        InstrType::Process,
        InstrIdent::SLE
    ),
    "SG" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_ge, compare_spawn_gt),
            argset!((Group, Item, Number) => spawn_item_num_ge, compare_spawn_gt),
        ],
        InstrType::Process,
        InstrIdent::SG
    ),
    "SGE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_geq, compare_spawn_ge),
            argset!((Group, Item, Number) => spawn_item_num_geq, compare_spawn_ge),
        ],
        InstrType::Process,
        InstrIdent::SGE
    ),
    "FE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_eq, compare_fork_eq),
            argset!((Group, Group, Item, Number) => fork_item_num_eq, compare_fork_eq),
        ],
        InstrType::Process,
        InstrIdent::FE
    ),
    "FNE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_ne, compare_fork_ne),
            argset!((Group, Group, Item, Number) => fork_item_num_ne, compare_fork_ne),
        ],
        InstrType::Process,
        InstrIdent::FNE
    ),
    "FL" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_le, compare_fork_lt),
            argset!((Group, Group, Item, Number) => fork_item_num_le, compare_fork_lt),
        ],
        InstrType::Process,
        InstrIdent::FL
    ),
    "FLE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_leq, compare_fork_le),
            argset!((Group, Group, Item, Number) => fork_item_num_leq, compare_fork_le),
        ],
        InstrType::Process,
        InstrIdent::FLE
    ),
    "FG" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_ge, compare_fork_gt),
            argset!((Group, Group, Item, Number) => fork_item_num_ge, compare_fork_gt),
        ],
        InstrType::Process,
        InstrIdent::FG
    ),
    "FGE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_geq, compare_fork_ge),
            argset!((Group, Group, Item, Number) => fork_item_num_geq, compare_fork_ge),
        ],
        InstrType::Process,
        InstrIdent::FGE
    ),
    "SRAND" => (
        false,
        &[argset!((Group, Number) => spawn_random, srand)],
        InstrType::Process,
        InstrIdent::SRAND,
    ),
    "FRAND" => (
        false,
        &[argset!((Group, Group, Number) => fork_random, frand)],
        InstrType::Process,
        InstrIdent::FRAND,
    ),
    "TSPAWN" => (
        false,
        &[argset!((Timer, Number, Number, Group) => tspawn, tspawn)],
        InstrType::Timer,
        InstrIdent::TSPAWN,
    ),
    "TSTART" => (
        false,
        &[argset!((Timer) => tstart, tstart)],
        InstrType::Timer,
        InstrIdent::TSTART,
    ),
    "TSTOP" => (
        false,
        &[argset!((Timer) => tstop, tstop)],
        InstrType::Timer,
        InstrIdent::TSTOP,
    ),
    "PAUSE" => (
        false,
        &[argset!((Group) => pause, pause)],
        InstrType::Process,
        InstrIdent::PAUSE,
    ),
    "RESUME" => (
        false,
        &[argset!((Group) => resume, resume)],
        InstrType::Process,
        InstrIdent::RESUME,
    ),
    "KILL" => (
        false,
        &[argset!((Group) => stop, kill)],
        InstrType::Process,
        InstrIdent::KILL,
    ),
    "TOGGLEON" => (
        false,
        &[argset!((Group) => ton, toggleon)],
        InstrType::Process,
        InstrIdent::TOGGLEON,
    ),
    "TOGGLEOFF" => (
        false,
        &[argset!((Group) => toff, toggleoff)],
        InstrType::Process,
        InstrIdent::TOGGLEOFF,
    ),
    "RAW" => (
        false,
        &[argset!((String) => raw_objs, skip)],
        InstrType::Special,
        InstrIdent::RAW,
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
