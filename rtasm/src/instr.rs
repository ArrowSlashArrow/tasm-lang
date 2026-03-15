use std::iter;

use gdlib::gdobj::{
    GDObjConfig, GDObject, Item, ItemType, ZLayer,
    misc::{default_block, text},
    triggers::{
        CompareOp, CompareOperand, DefaultMove, ItemAlign, MoveMode, MoveTarget, Op, RoundMode,
        SignMode, TargetMove, collision_block, collision_trigger, counter_object, item_compare,
        item_edit, move_trigger, persistent_item, random_trigger, spawn_trigger, time_control,
        toggle_trigger,
    },
};
use paste::paste;

// const GROUP_SPAWN_DELAY: f64 = 0.0044;
const GROUP_SPAWN_DELAY: f64 = 0.0044;

use crate::core::{
    HandlerArgs, HandlerData, HandlerFn, HandlerReturn, MemInfo, TasmParseError, TasmPrimitive,
    TasmValue, TasmValueType,
};

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
    &'static str,                     // ident
    bool,                             // exclusive to _init
    &[(&[TasmValueType], HandlerFn)], // handlers
)] = &[
    // inits
    ("MALLOC", true, &[argset!((Int) => malloc)]),
    ("FMALLOC", true, &[argset!((Int) => fmalloc)]),
    ("INITMEM", true, &[argset!([Number] => init_mem)]),
    ("PERS", true, &[argset!((Item) => pers)]),
    ("DISPLAY", true, &[argset!((Item) => display)]),
    ("IOBLOCK", true, &[argset!((Group, Int, String) => ioblock)]),
    // memory
    ("MFUNC", false, &[argset!(() => mfunc)]),
    ("MREAD", false, &[argset!(() => mread)]),
    ("MWRITE", false, &[argset!(() => mwrite)]),
    ("MPTR", false, &[argset!((Int) => mptr)]),
    ("MRESET", false, &[argset!(() => mreset)]),
    (
        "MOV",
        false,
        &[
            argset!((Item, Number) => arithmetic_item_num_mov),
            argset!((Item, Item) => arithmetic_2items_mov),
        ],
    ),
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
    ("TSTART", false, &[argset!((Timer) => tstart)]),
    ("TSTOP", false, &[argset!((Timer) => tstop)]),
];

macro_rules! wrap_objs {
    ($objs:expr) => {
        Ok(HandlerData::from_objects($objs))
    };
}

// utils
pub fn get_item_spec(item: &TasmValue) -> Option<Item> {
    match item {
        TasmValue::Counter(c) => Some(Item::Counter(*c)),
        TasmValue::Timer(t) => Some(Item::Timer(*t)),
        TasmValue::GDItem(i) => Some(*i),
        _ => None,
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

macro_rules! handlers {
    // handlers!((add, sub, mul, div) => _arith_2items)
    // variant: (arithmetic), [compare]; the var is lowercase and is converted.
    // for each argument, make a new fn that calls the inner fn
    // and returns the proper result type
    ( ($($var:ident),* $(,)?) => $inner_fn:ident) => {
        $(
            paste! {
                fn [<$inner_fn _ $var>](args: HandlerArgs) -> HandlerReturn {
                    Ok(HandlerData::from_objects($inner_fn(args, (LowerOp::$var).to_op(), false)))
                }
            }
        )*
    };

    ( [$($var:ident),* $(,)?] + $extra_groups:literal => $inner_fn:ident) => {
        $(
            paste! {
                fn [<$inner_fn _ $var>](args: HandlerArgs) -> HandlerReturn {
                    Ok(
                        HandlerData::from_objects($inner_fn(args, (LowerCompOp::$var).to_op()))
                            .extra_groups($extra_groups),
                    )
                }
            }
        )*
    };
}

// fn todo(_args: HandlerArgs) -> HandlerReturn {
//     unimplemented!()
// }

// useful for instructions that don't correspond to any objects
// namely debug instructions
// namely breakpoint
fn skip(_args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::default().skip_spaces(0))
}

/* WAIT */

fn nop(_args: HandlerArgs) -> HandlerReturn {
    // skip no-op space
    Ok(HandlerData::default().skip_spaces(1))
}

fn wait(args: HandlerArgs) -> HandlerReturn {
    // skip specified amount of spaces
    Ok(HandlerData::default().skip_spaces(args.args[0].to_int().unwrap()))
}

/* ARITHMETIC */

fn arithmetic_2items(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let result = get_item_spec(&args.args[0]).unwrap();
    let operand = get_item_spec(&args.args[1]).unwrap();
    vec![item_edit(
        &args.cfg,
        Some(operand),
        None,
        result,
        1.0,
        op,
        false,
        None,
        RoundMode::None,
        if round_res {
            RoundMode::Nearest
        } else {
            RoundMode::None
        },
        SignMode::None,
        SignMode::None,
    )]
}
fn arithmetic_3items(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();
    let op2 = get_item_spec(&args.args[2]).unwrap();
    vec![item_edit(
        &args.cfg,
        Some(op1),
        Some(op2),
        res,
        1.0,
        Op::Set,
        false,
        Some(op),
        RoundMode::None,
        if round_res {
            RoundMode::Nearest
        } else {
            RoundMode::None
        },
        SignMode::None,
        SignMode::None,
    )]
}
fn arithmetic_item_num(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    // second arg should always be a number
    let modifier = args.args[1].to_float().unwrap();
    vec![item_edit(
        &args.cfg,
        None,
        None,
        res,
        modifier,
        op,
        false,
        None,
        RoundMode::None,
        if round_res {
            RoundMode::Nearest
        } else {
            RoundMode::None
        },
        SignMode::None,
        SignMode::None,
    )]
}
fn arithmetic_2items_num(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();
    let mult = args.args[2].to_float().unwrap();
    vec![item_edit(
        &args.cfg,
        Some(op1),
        None,
        res,
        mult,
        Op::Set,
        // since we know this is only used for mul and div instructions, this is fine.
        op == Op::Mul,
        None,
        RoundMode::None,
        if round_res {
            RoundMode::Nearest
        } else {
            RoundMode::None
        },
        SignMode::None,
        SignMode::None,
    )]
}

handlers!((add, sub, mul, div, mov) => arithmetic_2items);
handlers!((add, sub, mul, div) => arithmetic_3items);
handlers!((add, sub, mul, div, mov) => arithmetic_item_num);
handlers!((mul, div) => arithmetic_2items_num);

// fldiv instructions are not supported in the macro, so they are defined here.
fn fldiv_2items(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_2items(args, Op::Div, true,))
}
fn fldiv_item_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_item_num(args, Op::Div, true,))
}
fn fldiv_3items(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_3items(args, Op::Div, true,))
}
fn fldiv_2items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_2items_num(args, Op::Div, true,))
}

/* COMPARES */

fn spawn_trg(spawn_cfg: &GDObjConfig, group: i16) -> GDObject {
    spawn_trigger(
        &spawn_cfg,
        group,
        GROUP_SPAWN_DELAY,
        0.0,
        false,
        true,
        false,
    )
}

fn spawn_item_num(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);

    let iargs = args.args;
    let lhs = get_item_spec(&iargs[1]).unwrap();

    let spawning_group = iargs[0].to_group_id().unwrap();
    let spawn_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([args.curr_group])
        .set_control_id(spawning_group as i32); // use auxiliary group for spawn trigger
    // SX rtn, I1, 42
    // args: [Group(n), ]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group, // spawn auxiliary group (spawn trigger)
            0,
            lhs.into(),
            CompareOperand::number_literal(iargs[2].to_float().unwrap()),
            op,
            0.0,
        ),
        spawn_trg(&spawn_cfg, spawning_group),
    ]
}
fn spawn_item_item(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);

    let iargs = args.args;
    let lhs = get_item_spec(&iargs[1]).unwrap();
    let rhs = get_item_spec(&iargs[2]).unwrap();
    let spawning_group = iargs[0].to_group_id().unwrap();
    let spawn_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([args.curr_group])
        .set_control_id(spawning_group as i32); // use auxiliary group for spawn trigger
    // SX rtn, I1, 42
    // args: [Group(n), ]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group, // spawn auxiliary group (spawn trigger)
            0,
            lhs.into(),
            rhs.into(),
            op,
            0.0,
        ),
        spawn_trg(&spawn_cfg, spawning_group),
    ]
}
fn fork_item_num(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    // below
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1).scale(0.33, 0.33);

    let iargs = args.args;
    let lhs = get_item_spec(&iargs[2]).unwrap();
    let num = iargs[3].to_float().unwrap();

    let spawning_true = iargs[0].to_group_id().unwrap();
    let spawning_false = iargs[1].to_group_id().unwrap();
    let spawn_true_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group])
        .set_control_id(spawning_true as i32); // use auxiliary group for spawn trigger

    let spawn_false_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 - 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group + 1])
        .set_control_id(spawning_false as i32); // use auxiliary group for spawn trigger
    // FX rtn, rtn2, I1, 42
    // args: [Group(n), Group(n), Item, Number]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group,     // spawn auxiliary group (true trigger)
            args.curr_group + 1, // spawn 2nd aux group (false trigger)
            lhs.into(),
            CompareOperand::number_literal(num),
            op,
            0.0,
        ),
        spawn_trg(&spawn_true_cfg, spawning_true),
        spawn_trg(&spawn_false_cfg, spawning_false),
    ]
}
fn fork_item_item(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    // below
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1).scale(0.33, 0.33);

    let iargs = args.args;
    let lhs = get_item_spec(&iargs[2]).unwrap();
    let rhs = get_item_spec(&iargs[3]).unwrap();

    let spawning_true = iargs[0].to_group_id().unwrap();
    let spawning_false = iargs[1].to_group_id().unwrap();
    let spawn_true_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group])
        .set_control_id(spawning_true as i32); // use auxiliary group for spawn trigger

    let spawn_false_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 - 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group + 1])
        .set_control_id(spawning_false as i32); // use auxiliary group for spawn trigger
    // FX rtn, rtn2, I1, 42
    // args: [Group(n), Group(n), Item, Item]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group,     // spawn auxiliary group (true trigger)
            args.curr_group + 1, // spawn 2nd aux group (false trigger)
            lhs.into(),
            rhs.into(),
            op,
            0.0,
        ),
        spawn_trg(&spawn_true_cfg, spawning_true),
        spawn_trg(&spawn_false_cfg, spawning_false),
    ]
}

handlers!([eq, ne, le, leq, ge, geq] + 1 => spawn_item_num);
handlers!([eq, ne, le, leq, ge, geq] + 1 => spawn_item_item);
handlers!([eq, ne, le, leq, ge, geq] + 2 => fork_item_num);
handlers!([eq, ne, le, leq, ge, geq] + 2 => fork_item_item);

/* RANDOMS */

fn spawn_random(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let random_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);

    let iargs = args.args;
    let spawning_group = iargs[0].to_group_id().unwrap();
    let chance = (&iargs[1]).to_float().unwrap();

    let aux_group = args.curr_group;
    let spawn_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([aux_group])
        .set_control_id(spawning_group as i32); // use auxiliary group for spawn trigger

    Ok(HandlerData::from_objects(vec![
        random_trigger(&random_cfg, chance, aux_group, 0),
        spawn_trg(&spawn_cfg, spawning_group),
    ])
    .extra_groups(1))
}

fn fork_random(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let random_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);

    let iargs = args.args;
    let spawning_group1 = iargs[0].to_group_id().unwrap();
    let spawning_group2 = iargs[1].to_group_id().unwrap();
    let chance = (&iargs[2]).to_float().unwrap();

    let aux_group1 = args.curr_group;
    let aux_group2 = args.curr_group + 1;
    let spawn_cfg1 = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([aux_group1])
        .set_control_id(spawning_group1 as i32); // use auxiliary group for spawn trigger
    let spawn_cfg2 = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 - 7.5)
        .scale(0.5, 0.5)
        .groups([aux_group2])
        .set_control_id(spawning_group2 as i32); // use auxiliary group for spawn trigger

    Ok(HandlerData::from_objects(vec![
        random_trigger(&random_cfg, chance, aux_group1, aux_group2),
        spawn_trg(&spawn_cfg1, spawning_group1),
        spawn_trg(&spawn_cfg2, spawning_group2),
    ])
    .extra_groups(2))
}

/* PROCESS */

fn spawn(args: HandlerArgs) -> HandlerReturn {
    let spawning_group = args.args[0].to_group_id().unwrap();
    let cfg = args.cfg.set_control_id(spawning_group as i32);
    wrap_objs!(vec![spawn_trigger(
        &cfg,
        spawning_group,
        GROUP_SPAWN_DELAY,
        0.0,
        false,
        true,
        false,
    )])
}

/* TIMERS */

fn tstart(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![time_control(
        &args.cfg,
        get_item_spec(&args.args[0]).unwrap().id(),
        false,
    )]))
}

fn tstop(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![time_control(
        &args.cfg,
        get_item_spec(&args.args[0]).unwrap().id(),
        true,
    )]))
}

/* MEMORY */

fn mptr(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let move_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 + 7.5);
    let add_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 - 7.5);
    let move_amount = args.args[0].to_float().unwrap();
    let invalid_move_reason;
    let is_valid_mem_move = match args.mem_info {
        Some(mem) => {
            if move_amount as i16 <= mem.size {
                invalid_move_reason = String::new();
                true
            } else {
                invalid_move_reason = "Pointer moved more spaces than memory size".into();
                false
            }
        }
        None => {
            invalid_move_reason = "Pointer moved while no memory exists".into();
            false
        }
    };

    if is_valid_mem_move {
        Ok(HandlerData::from_objects(vec![
            move_trigger(
                &move_cfg,
                MoveMode::Default(DefaultMove {
                    dx: 30.0 * move_amount,
                    dy: 0.0,
                    x_lock: None,
                    y_lock: None,
                }),
                0.0,
                args.ptr_group,
                false,
                false,
                None,
            ),
            item_edit(
                &add_cfg,
                None,
                None,
                Item::Counter(args.ptrpos_id),
                move_amount,
                Op::Add,
                false,
                None,
                RoundMode::None,
                RoundMode::None,
                SignMode::None,
                SignMode::None,
            ),
        ]))
    } else {
        Err(TasmParseError::InvalidPointerMove(
            invalid_move_reason,
            args.line,
        ))
    }
}

fn mreset(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let move_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 + 7.5);
    let add_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 - 7.5);
    Ok(HandlerData::from_objects(vec![
        move_trigger(
            &move_cfg,
            MoveMode::Targeting(TargetMove {
                target_group_id: MoveTarget::Group(args.ptr_reset_group),
                center_group_id: None,
                axis_only: None,
            }),
            0.0,
            args.ptr_group,
            false,
            false,
            None,
        ),
        item_edit(
            &add_cfg,
            None,
            None,
            Item::Counter(args.ptrpos_id),
            0.0,
            Op::Set,
            false,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ),
    ]))
}

fn mfunc(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![move_trigger(
        &args.cfg,
        MoveMode::Default(DefaultMove {
            dx: 0.0,
            dy: 30.0,
            x_lock: None,
            y_lock: None,
        }),
        0.0,
        args.ptr_group,
        false,
        false,
        None,
    )])
    .skip_spaces(2))
}

fn mem_mode(args: HandlerArgs, toggle_read: bool) -> HandlerReturn {
    let top_cfg = args.cfg.clone().scale(0.5, 0.5).y(args.cfg.pos.1 + 7.5);
    let bottom_cfg = args.cfg.clone().scale(0.5, 0.5).y(args.cfg.pos.1 - 7.5);
    let mem_info = args.mem_info.unwrap();

    Ok(HandlerData::from_objects(vec![
        toggle_trigger(&top_cfg, mem_info.write_group, !toggle_read),
        toggle_trigger(&bottom_cfg, mem_info.read_group, toggle_read),
    ]))
}

fn mwrite(args: HandlerArgs) -> HandlerReturn {
    mem_mode(args, false)
}
fn mread(args: HandlerArgs) -> HandlerReturn {
    mem_mode(args, true)
}

/* INITS */

fn display(args: HandlerArgs) -> HandlerReturn {
    let item = get_item_spec(&args.args[0]).unwrap();
    let cfg = GDObjConfig::new()
        .pos(-75.0, 75.0 + 30.0 * args.displayed_items as f64)
        .scale(0.5, 0.5);

    let obj = counter_object(&cfg, item, ItemAlign::Center, false);

    Ok(HandlerData::from_objects(vec![obj])
        .skip_spaces(0)
        .added_item_display())
}

pub fn ioblock(args: HandlerArgs) -> HandlerReturn {
    let spawn_group = args.args[0].to_group_id().unwrap();
    let position = args.args[1].to_int().unwrap();
    let msg = args.args[2].to_string().unwrap();
    let cfg = GDObjConfig::new().pos(75.0 + position as f64 * 30.0, 75.0);
    let text_cfg = cfg.clone().scale(0.25, 0.25).set_z_layer(ZLayer::T2);
    let spawn_cfg = cfg.clone().touchable(true).multitrigger(true);

    Ok(HandlerData::from_objects(vec![
        default_block(&cfg),
        spawn_trigger(
            &spawn_cfg,
            spawn_group,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
        ),
        text(&text_cfg, msg, 0),
    ])
    .skip_spaces(0))
}

fn pers(args: HandlerArgs) -> HandlerReturn {
    let item = get_item_spec(&args.args[0]).unwrap();
    Ok(HandlerData::from_objects(vec![persistent_item(
        &args.cfg,
        item.id(),
        item.get_type() == ItemType::Timer,
        true,
        false,
        false,
    )]))
}

/* MEM INITS */

fn malloc_inner(args: HandlerArgs, float_mem: bool) -> HandlerData {
    let (mem_x, mem_y) = (45.0, 165.0 + args.routine_count as f64 * 30.0);
    let mem_size = args.args[0].to_int().unwrap() as i16;

    let start_counter_id = args.ptrpos_id - mem_size - 1;
    let ptr_collblock_id = mem_size + 1;
    let memreg_id = args.ptrpos_id - 1;

    let mut next_free_group = args.curr_group;

    let ptr_reset_group = next_free_group;
    let ptr_group = next_free_group + 1;
    next_free_group += 2;

    let read_group = next_free_group;
    let write_group = next_free_group + 1;
    next_free_group += 2;

    let block_cfg = &GDObjConfig::new()
        .pos(mem_x, mem_y - 30.0)
        .scale(0.5, 0.5)
        .groups([ptr_reset_group]);

    let mut objs = vec![
        // reset block
        default_block(&block_cfg),
        // pointer block
        collision_block(
            &block_cfg.clone().groups([ptr_group]).scale(0.8, 0.8),
            ptr_collblock_id,
            true,
        ),
    ];

    let mut idx = 0i16;
    let mut counter_id = start_counter_id;

    let memreg_item = match float_mem {
        true => Item::Timer(memreg_id),
        false => Item::Counter(memreg_id),
    };
    let ptrpos_item = Item::Counter(args.ptrpos_id);

    while counter_id < memreg_id {
        let item_group = next_free_group;
        let collblock_id = idx + 1;
        let xpos = idx as f64 * 30.0 + mem_x;

        let counter_item = match float_mem {
            true => Item::Timer(counter_id),
            false => Item::Counter(counter_id),
        };

        let mut cfg = GDObjConfig::new().pos(xpos, mem_y);

        objs.push(collision_block(&cfg, collblock_id, false));
        cfg = cfg
            .pos(mem_x - 71.25, mem_y + (idx + 1) as f64 * 7.5 - 18.75)
            .groups([item_group])
            .scale(0.25, 0.25);
        objs.push(collision_trigger(
            &cfg,
            collblock_id,
            ptr_collblock_id,
            item_group,
            false,
            false,
            false,
            true,
            false,
        ));
        cfg = cfg
            .pos(xpos, mem_y + 30.0)
            .groups([item_group, write_group])
            .spawnable(true)
            .multitrigger(true)
            .scale(1.0, 1.0);
        // write memreg to item
        objs.push(item_edit(
            &cfg,
            Some(memreg_item),
            None,
            counter_item,
            1.0,
            Op::Set,
            false,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));
        // read item to memreg
        cfg = cfg.y(mem_y + 60.0).groups([item_group, read_group]);
        // write memreg to item
        objs.push(item_edit(
            &cfg,
            Some(counter_item),
            None,
            memreg_item,
            1.0,
            Op::Set,
            false,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));
        // moves ptr back after is moves up
        cfg = cfg.y(mem_y + 90.0).groups([item_group]);
        objs.push(move_trigger(
            &cfg,
            MoveMode::Default(DefaultMove {
                dx: 0.0,
                dy: -30.0,
                x_lock: None,
                y_lock: None,
            }),
            0.0,
            ptr_group,
            false,
            false,
            None,
        ));

        // counter obj
        cfg = cfg
            .y(mem_y - 60.0)
            .groups(iter::empty::<i16>())
            .scale(0.4, 0.4)
            .angle(-30.0);
        objs.push(counter_object(&cfg, counter_item, ItemAlign::Center, false));

        next_free_group += 1;
        counter_id += 1;
        idx += 1;
    }

    objs.extend_from_slice(&[
        // memreg and ptrpos counters
        counter_object(
            &GDObjConfig::new()
                .pos(mem_x + mem_size as f64 * 30.0, mem_y - 60.0)
                .scale(0.4, 0.4)
                .angle(-30.0),
            memreg_item,
            ItemAlign::Center,
            false,
        ),
        counter_object(
            &GDObjConfig::new()
                .pos(mem_x + (mem_size + 1) as f64 * 30.0, mem_y - 60.0)
                .scale(0.4, 0.4)
                .angle(-30.0),
            ptrpos_item,
            ItemAlign::Center,
            false,
        ),
        // memory text
        text(
            &GDObjConfig::new().pos(mem_x, mem_y + 150.0).scale(0.5, 0.5),
            "memory",
            0,
        ),
    ]);

    // 1. each memory cell gets a column
    // 2. mem ptr and mem ptr reset <- dont forget to include these in return
    // 3. memory text
    // 4. memreg and ptrpos counters
    // 5. return memtype, used groups (next free - args.current), ptr reset group, meminfo

    let mut data = HandlerData::from_objects(objs);
    data.used_extra_groups = next_free_group - args.curr_group;
    data.ptr_reset_group = ptr_reset_group;
    data.ptr_group = ptr_group;
    data.new_mem = Some(MemInfo {
        _type: match float_mem {
            true => crate::core::MemType::Float,
            false => crate::core::MemType::Int,
        },
        memreg: match float_mem {
            true => TasmValue::Timer(memreg_id),
            false => TasmValue::Counter(memreg_id),
        },
        size: mem_size,
        ptrpos: TasmValue::Counter(args.ptrpos_id),
        read_group,
        write_group,
        start_counter_id,
    });

    data
}

fn malloc(args: HandlerArgs) -> HandlerReturn {
    Ok(malloc_inner(args, false))
}

fn fmalloc(args: HandlerArgs) -> HandlerReturn {
    Ok(malloc_inner(args, true))
}

fn init_mem(args: HandlerArgs) -> HandlerReturn {
    let y_offset = args.routine_count as f64 * 30.0 + 150.0;
    let mut cfg = GDObjConfig::new().pos(-15.0, 0.0).scale(0.25, 0.25);

    let mem_info = args.mem_info.unwrap();
    let start_counter = mem_info.start_counter_id;

    let mut objs = vec![];

    for (idx, v) in args.args.iter().enumerate() {
        cfg = cfg.y(y_offset + 7.5 * (idx + 1) as f64 - 18.75);

        objs.push(item_edit(
            &cfg,
            None,
            None,
            match mem_info._type {
                crate::core::MemType::Float => Item::Timer(start_counter + idx as i16),
                crate::core::MemType::Int => Item::Counter(start_counter + idx as i16),
            },
            v.to_float().unwrap(),
            Op::Set,
            false,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));
    }

    Ok(HandlerData::from_objects(objs))
}
