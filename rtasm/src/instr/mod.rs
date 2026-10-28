use gdlib::cclocallevels::gdobj::structs::{CompareOp, Item, Op};

use phf::phf_map;

use crate::{
    core::{
        HandlerFn, HandlerReturn,
        error::{TasmError, TasmErrorType},
        flags::FlagValue,
        structs::{HandlerArgs, InstrType, TasmPrimitive, TasmValue},
    },
    instr::fns::*,
};

pub mod fns;

/// Length of 1 game tick in seconds.
pub const GROUP_SPAWN_DELAY: f64 = 0.0044;

// convert a list of type identifiers into a slice
macro_rules! argset {
    (($($arg:ident),*) => $fn:ident) => {
        (&[ $(TasmPrimitive::$arg,)* ], $fn)
    };

    // // use this for list args
    // ([$argtype:ident] => $fn:ident) => {
    //     (&[TasmValueType::List(TasmPrimitive::$argtype)], $fn)
    // }
}

/// This function exists as a placeholder for external symbols. Any instructions that use this function are to be resolved in the linking stage.
/// This function WILL PANIC when called. DO NOT call this in `tasm.handle_routines`.
pub fn placeholder_panic_fn(args: HandlerArgs) -> HandlerReturn {
    Err(TasmError {
        etype: TasmErrorType::InvalidInstruction,
        file: String::new(),
        routine: String::new(),
        line: args.line,
        details: format!("Placeholder panic function was hit!"),
        errcode: -1,
    })
}

pub type HandlerAssoc = (&'static [TasmPrimitive], HandlerFn);
pub type Handlers = &'static [HandlerAssoc];
// mnemonic => (is init, argsets, instr type, flags)
pub const INSTR_SPEC: phf::Map<&'static str, (bool, Handlers, InstrType, &[&str])> = phf_map! {
    // inits
    "PERS" => (false, &[argset!((Item) => pers)], InstrType::Misc, &[]),
    "UNPERS" => (false, &[argset!((Item) => unpers)], InstrType::Misc, &[]),
    "UNPERSALL" => (false, &[argset!(() => unpersall)], InstrType::Misc, &[]),
    "RPERSALL" => (false, &[argset!(() => rpersall)], InstrType::Misc, &[]),
    // if an instruction can only go in the _init routine, it **MUST** be designated that.
    "DISPLAY" => (
        true,
        &[argset!((Item) => display)],
        InstrType::Init, &[]
    ),
    "IOBLOCK" => (
        true,
        &[argset!((Group, Int, String) => ioblock)],
        InstrType::Init, &[]
    ),
    "MOV" => (
        false,
        &[
            argset!((Item, Number) => arithmetic_item_num_mov),
            argset!((Item, Item) => arithmetic_2items_mov),
        ],
        InstrType::Arithmetic, &["itemmod", "resmode", "finmode", "op", "iter", "divmod"]
    ),
    // debug
    "BREAKPOINT" => (
        false,
        &[argset!(() => skip)],
        InstrType::Debug,
        &[]
    ),
    // Process
    "SPAWN" => (
        false,
        &[argset!((Group) => spawn)],
        InstrType::Process,
        &["ordered", "noremap", "remap", "delay"]
    ),
    // Waits
    "NOP" => (false, &[argset!(() => nop)], InstrType::Wait, &[]),
    "WAIT" => (false, &[argset!((Int) => wait)], InstrType::Wait, &[]),
    "WAITS" => (
        false,
        &[argset!((Number) => waits)],
        InstrType::Wait, &[]
    ),
    // Arithmetic
    "ADD" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_add),
            argset!((Item, Number) => arithmetic_item_num_add),
            argset!((Item, Item, Item) => arithmetic_3items_add),
        ],
        InstrType::Arithmetic, &["itemmod", "resmode", "finmode", "op", "iter", "divmod"]
    ),
    "SUB" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_sub),
            argset!((Item, Number) => arithmetic_item_num_sub),
            argset!((Item, Item, Item) => arithmetic_3items_sub),
        ],
        InstrType::Arithmetic, &["itemmod", "resmode", "finmode", "op", "iter", "divmod"]
    ),
    "MUL" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_mul),
            argset!((Item, Number) => arithmetic_item_num_mul),
            argset!((Item, Item, Item) => arithmetic_3items_mul),
            argset!((Item, Item, Number) => arithmetic_2items_num_mul),
        ],
        InstrType::Arithmetic, &["itemmod", "resmode", "finmode", "op", "iter", "divmod"]
    ),
    "DIV" => (
        false,
        &[
            argset!((Item, Item) => arithmetic_2items_div),
            argset!((Item, Number) => arithmetic_item_num_div),
            argset!((Item, Item, Item) => arithmetic_3items_div),
            argset!((Item, Item, Number) => arithmetic_2items_num_div),
        ],
        InstrType::Arithmetic, &["itemmod", "resmode", "finmode", "op", "iter", "divmod"]
    ),
    "FLDIV" => (
        false,
        &[
            argset!((Item, Item) => fldiv_2items),
            argset!((Item, Number) => fldiv_item_num),
            argset!((Item, Item, Item) => fldiv_3items),
            argset!((Item, Item, Number) => fldiv_2items_num),
        ],
        InstrType::Arithmetic, &["itemmod", "resmode", "finmode", "op", "iter", "divmod"]
    ),
    "SE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_eq),
            argset!((Group, Item, Number) => spawn_item_num_eq),
        ],
        InstrType::Process, &["ordered", "noremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "SNE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_ne),
            argset!((Group, Item, Number) => spawn_item_num_ne),
        ],
        InstrType::Process, &["ordered", "noremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "SL" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_le),
            argset!((Group, Item, Number) => spawn_item_num_le),
        ],
        InstrType::Process, &["ordered", "noremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "SLE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_leq),
            argset!((Group, Item, Number) => spawn_item_num_leq),
        ],
        InstrType::Process, &["ordered", "noremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "SG" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_ge),
            argset!((Group, Item, Number) => spawn_item_num_ge),
        ],
        InstrType::Process, &["ordered", "noremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "SGE" => (
        false,
        &[
            argset!((Group, Item, Item) => spawn_item_item_geq),
            argset!((Group, Item, Number) => spawn_item_num_geq),
        ],
        InstrType::Process, &["ordered", "noremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "FE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_eq),
            argset!((Group, Group, Item, Number) => fork_item_num_eq),
        ],
        InstrType::Process, &["ordered", "lnoremap", "rnoremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "FNE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_ne),
            argset!((Group, Group, Item, Number) => fork_item_num_ne),
        ],
        InstrType::Process, &["ordered", "lnoremap", "rnoremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "FL" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_le),
            argset!((Group, Group, Item, Number) => fork_item_num_le),
        ],
        InstrType::Process, &["ordered", "lnoremap", "rnoremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "FLE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_leq),
            argset!((Group, Group, Item, Number) => fork_item_num_leq),
        ],
        InstrType::Process, &["ordered", "lnoremap", "rnoremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "FG" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_ge),
            argset!((Group, Group, Item, Number) => fork_item_num_ge),
        ],
        InstrType::Process, &["ordered", "lnoremap", "rnoremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "FGE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => fork_item_item_geq),
            argset!((Group, Group, Item, Number) => fork_item_num_geq),
        ],
        InstrType::Process, &["ordered", "lnoremap", "rnoremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "SRAND" => (
        false,
        &[argset!((Group, Number) => spawn_random)],
        InstrType::Process, &["ordered", "noremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "FRAND" => (
        false,
        &[argset!((Group, Group, Number) => fork_random)],
        InstrType::Process, &["ordered", "lnoremap", "rnoremap", "lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "ISE" => (
        false,
        &[
            argset!((Group, Item, Item) => instant_spawn_item_item_eq),
            argset!((Group, Item, Number) => instant_spawn_item_num_eq),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "ISNE" => (
        false,
        &[
            argset!((Group, Item, Item) => instant_spawn_item_item_ne),
            argset!((Group, Item, Number) => instant_spawn_item_num_ne),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "ISL" => (
        false,
        &[
            argset!((Group, Item, Item) => instant_spawn_item_item_le),
            argset!((Group, Item, Number) => instant_spawn_item_num_le),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "ISLE" => (
        false,
        &[
            argset!((Group, Item, Item) => instant_spawn_item_item_leq),
            argset!((Group, Item, Number) => instant_spawn_item_num_leq),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "ISG" => (
        false,
        &[
            argset!((Group, Item, Item) => instant_spawn_item_item_ge),
            argset!((Group, Item, Number) => instant_spawn_item_num_ge),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "ISGE" => (
        false,
        &[
            argset!((Group, Item, Item) => instant_spawn_item_item_geq),
            argset!((Group, Item, Number) => instant_spawn_item_num_geq),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "IFE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => instant_fork_item_item_eq),
            argset!((Group, Group, Item, Number) => instant_fork_item_num_eq),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "IFNE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => instant_fork_item_item_ne),
            argset!((Group, Group, Item, Number) => instant_fork_item_num_ne),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "IFL" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => instant_fork_item_item_le),
            argset!((Group, Group, Item, Number) => instant_fork_item_num_le),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "IFLE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => instant_fork_item_item_leq),
            argset!((Group, Group, Item, Number) => instant_fork_item_num_leq),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "IFG" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => instant_fork_item_item_ge),
            argset!((Group, Group, Item, Number) => instant_fork_item_num_ge),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "IFGE" => (
        false,
        &[
            argset!((Group, Group, Item, Item) => instant_fork_item_item_geq),
            argset!((Group, Group, Item, Number) => instant_fork_item_num_geq),
        ],
        InstrType::Process, &["lmod", "lmodop", "lmode", "rmod", "rmodop", "rmode"]
    ),
    "ISRAND" => (
        false,
        &[argset!((Group, Number) => instant_spawn_random)],
        InstrType::Process, &[]
    ),
    "IFRAND" => (
        false,
        &[argset!((Group, Group, Number) => instant_fork_random)],
        InstrType::Process, &[]
    ),
    "TSPAWN" => (
        false,
        &[argset!((Timer, Number, Number, Group) => tspawn)],
        InstrType::Timer, &["tstop", "tmod", "tpaused", "nover"]
    ),
    "TSTART" => (
        false,
        &[argset!((Timer) => tstart)],
        InstrType::Timer, &[]
    ),
    "TSTOP" => (
        false,
        &[argset!((Timer) => tstop)],
        InstrType::Timer, &[]
    ),
    "PAUSE" => (
        false,
        &[argset!((Group) => pause)],
        InstrType::Process, &[]
    ),
    "RESUME" => (
        false,
        &[argset!((Group) => resume)],
        InstrType::Process, &[]
    ),
    "KILL" => (
        false,
        &[argset!((Group) => stop)],
        InstrType::Process, &[]
    ),
    "TOGGLEON" => (
        false,
        &[argset!((Group) => ton)],
        InstrType::Process, &[]
    ),
    "TOGGLEOFF" => (
        false,
        &[argset!((Group) => toff)],
        InstrType::Process, &[]
    ),
    "RAW" => (
        false,
        &[argset!((String) => raw_objs)],
        InstrType::Misc, &[]
    ),
    "RAWTRG" => (
        false,
        &[argset!((String) => raw_trigger)],
        InstrType::Misc, &[]
    ),
    // instant collision check between two hitboxes. spawns first group if colliding, spawns second if not
    "INSTCOLL" => (
        false,
        &[argset!((Group, Group, CollBlock, CollBlock) => instcoll)],
        InstrType::Process, &[]
    ),
    // collision event listener
    "COLL" => (
        false,
        &[argset!((Group, CollBlock, CollBlock) => coll)],
        InstrType::Process, &["nospawn", "noexit"]
    ),
    // event listener
    "EVENT" => (
        true,
        &[argset!((Group, Int) => event)],
        InstrType::Process, &["extra1", "extra2"]
    )
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

/// Returns the value of the flag if it was given as a flag argument. Returns the `default` value if it was not passed.
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
