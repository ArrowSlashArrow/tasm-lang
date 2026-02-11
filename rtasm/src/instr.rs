use gdlib::gdobj::{
    GDObjConfig, GDObject,
    misc::{default_block, text},
    triggers::{
        CompareOp, CounterMode, DefaultMove, ItemAlign, ItemType, MoveMode, Op, RoundMode,
        SignMode, TargetMove, counter_object, item_compare, item_edit, move_trigger, spawn_trigger,
        toggle_trigger,
    },
};
use paste::paste;

// const GROUP_SPAWN_DELAY: f64 = 0.0044;
const GROUP_SPAWN_DELAY: f64 = 0.0044;

use crate::core::{
    HandlerArgs, HandlerData, HandlerFn, HandlerReturn, TasmPrimitive, TasmValue, TasmValueType,
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
    ("MALLOC", true, &[argset!((Int) => todo)]),
    ("FMALLOC", true, &[argset!((Int) => todo)]),
    ("INITMEM", true, &[argset!([Number] => todo)]),
    ("PERS", true, &[argset!((Item) => todo)]),
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
    // Commented out due to being non-v0.1.0
    // ("WAIT", false, &[argset!((Int) => wait)]),

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
];

// utils
fn get_item_spec(item: &TasmValue) -> Option<(i16, ItemType)> {
    match item {
        TasmValue::Counter(c) => Some((*c, ItemType::Counter)),
        TasmValue::Timer(t) => Some((*t, ItemType::Timer)),
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
    pub fn to_op(&self) -> Op {
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
    pub fn to_op(&self) -> CompareOp {
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

fn todo(_args: HandlerArgs) -> HandlerReturn {
    unimplemented!()
}

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

// fn wait(args: HandlerArgs) -> HandlerReturn {
//     // skip specified amount of spaces
//     Ok(HandlerData::default().skip_spaces(args.args[0].to_int().unwrap()))
// }

/* ARITHMETIC */

fn arithmetic_2items(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let (res_id, res_t) = get_item_spec(&args.args[0]).unwrap();
    let (op_id, op_t) = get_item_spec(&args.args[1]).unwrap();
    vec![item_edit(
        &args.cfg,
        Some((op_id as i32, op_t)),
        None,
        res_id,
        res_t,
        1.0,
        op,
        None,
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
    let (res_id, res_t) = get_item_spec(&args.args[0]).unwrap();
    let (op1_id, op1_t) = get_item_spec(&args.args[1]).unwrap();
    let (op2_id, op2_t) = get_item_spec(&args.args[2]).unwrap();
    vec![item_edit(
        &args.cfg,
        Some((op1_id as i32, op1_t)),
        Some((op2_id as i32, op2_t)),
        res_id,
        res_t,
        1.0,
        Op::Set,
        None,
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
    let (res_id, res_t) = get_item_spec(&args.args[0]).unwrap();
    // second arg should always be a number
    let modifier = args.args[1].to_float().unwrap();
    vec![item_edit(
        &args.cfg,
        None,
        None,
        res_id,
        res_t,
        modifier,
        op,
        None,
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
    let (res_id, res_t) = get_item_spec(&args.args[0]).unwrap();
    let (op1_id, op1_t) = get_item_spec(&args.args[1]).unwrap();
    let mult = args.args[2].to_float().unwrap();
    vec![item_edit(
        &args.cfg,
        Some((op1_id as i32, op1_t)),
        None,
        res_id,
        res_t,
        mult,
        Op::Set,
        Some(op),
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
    Ok(HandlerData::from_objects(arithmetic_2items(
        args,
        Op::Div,
        true,
    )))
}
fn fldiv_item_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(arithmetic_item_num(
        args,
        Op::Div,
        true,
    )))
}
fn fldiv_3items(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(arithmetic_3items(
        args,
        Op::Div,
        true,
    )))
}
fn fldiv_2items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(arithmetic_2items_num(
        args,
        Op::Div,
        true,
    )))
}

/* COMPARES */

fn spawn_item_num(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    // below
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);
    let spawn_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([args.curr_group]); // use auxiliary group for spawn trigger

    let iargs = args.args;
    let (lhs_id, lhs_t) = get_item_spec(&iargs[1]).unwrap();
    // SX rtn, I1, 42
    // args: [Group(n), ]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group, // spawn auxiliary group (spawn trigger)
            0,
            (
                lhs_id as i32,
                lhs_t,
                1.0,
                Op::Mul,
                RoundMode::None,
                SignMode::None,
            ),
            (
                0,
                ItemType::Counter,
                iargs[2].to_float().unwrap(),
                Op::Mul,
                RoundMode::None,
                SignMode::None,
            ),
            op,
            0.0,
        ),
        spawn_trigger(
            &spawn_cfg,
            iargs[0].to_group_id().unwrap() as i32,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
        ),
    ]
}
fn spawn_item_item(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    // below
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);
    let spawn_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([args.curr_group]); // use auxiliary group for spawn trigger

    let iargs = args.args;
    let (lhs_id, lhs_t) = get_item_spec(&iargs[1]).unwrap();
    let (rhs_id, rhs_t) = get_item_spec(&iargs[2]).unwrap();
    // SX rtn, I1, 42
    // args: [Group(n), ]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group, // spawn auxiliary group (spawn trigger)
            0,
            (
                lhs_id as i32,
                lhs_t,
                1.0,
                Op::Mul,
                RoundMode::None,
                SignMode::None,
            ),
            (
                rhs_id as i32,
                rhs_t,
                1.0,
                Op::Mul,
                RoundMode::None,
                SignMode::None,
            ),
            op,
            0.0,
        ),
        spawn_trigger(
            &spawn_cfg,
            iargs[0].to_group_id().unwrap() as i32,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
        ),
    ]
}
fn fork_item_num(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    // below
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1).scale(0.33, 0.33);
    let spawn_true_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group]); // use auxiliary group for spawn trigger

    let spawn_false_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 - 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group + 1]); // use auxiliary group for spawn trigger

    let iargs = args.args;
    let (lhs_id, lhs_t) = get_item_spec(&iargs[2]).unwrap();
    let num = iargs[3].to_float().unwrap();
    // FX rtn, rtn2, I1, 42
    // args: [Group(n), Group(n), Item, Number]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group,     // spawn auxiliary group (true trigger)
            args.curr_group + 1, // spawn 2nd aux group (false trigger)
            (
                lhs_id as i32,
                lhs_t,
                1.0,
                Op::Mul,
                RoundMode::None,
                SignMode::None,
            ),
            (
                0,
                ItemType::Counter,
                num,
                Op::Mul,
                RoundMode::None,
                SignMode::None,
            ),
            op,
            0.0,
        ),
        spawn_trigger(
            &spawn_true_cfg,
            // spawn true group
            iargs[0].to_group_id().unwrap() as i32,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
        ),
        spawn_trigger(
            &spawn_false_cfg,
            // spawn false group
            iargs[1].to_group_id().unwrap() as i32,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
        ),
    ]
}
fn fork_item_item(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    // below
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1).scale(0.33, 0.33);
    let spawn_true_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group]); // use auxiliary group for spawn trigger

    let spawn_false_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 - 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group + 1]); // use auxiliary group for spawn trigger

    let iargs = args.args;
    let (lhs_id, lhs_t) = get_item_spec(&iargs[2]).unwrap();
    let (rhs_id, rhs_t) = get_item_spec(&iargs[3]).unwrap();
    // FX rtn, rtn2, I1, 42
    // args: [Group(n), Group(n), Item, Item]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group,     // spawn auxiliary group (true trigger)
            args.curr_group + 1, // spawn 2nd aux group (false trigger)
            (
                lhs_id as i32,
                lhs_t,
                1.0,
                Op::Mul,
                RoundMode::None,
                SignMode::None,
            ),
            (
                rhs_id as i32,
                rhs_t,
                1.0,
                Op::Mul,
                RoundMode::None,
                SignMode::None,
            ),
            op,
            0.0,
        ),
        spawn_trigger(
            &spawn_true_cfg,
            // spawn true group
            iargs[0].to_group_id().unwrap() as i32,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
        ),
        spawn_trigger(
            &spawn_false_cfg,
            // spawn false group
            iargs[1].to_group_id().unwrap() as i32,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
        ),
    ]
}

handlers!([eq, ne, le, leq, ge, geq] + 1 => spawn_item_num);
handlers!([eq, ne, le, leq, ge, geq] + 1 => spawn_item_item);
handlers!([eq, ne, le, leq, ge, geq] + 2 => fork_item_num);
handlers!([eq, ne, le, leq, ge, geq] + 2 => fork_item_item);

fn spawn(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![spawn_trigger(
        &args.cfg,
        args.args[0].to_group_id().unwrap() as i32,
        GROUP_SPAWN_DELAY,
        0.0,
        false,
        true,
        false,
    )]))
}

fn mptr(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let move_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 + 7.5);
    let add_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 - 7.5);
    let move_amount = args.args[0].to_float().unwrap();
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
            args.ptrpos_id,
            ItemType::Counter,
            move_amount,
            Op::Add,
            None,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ),
    ]))
}

fn mreset(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let move_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 + 7.5);
    let add_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 - 7.5);
    Ok(HandlerData::from_objects(vec![
        move_trigger(
            &move_cfg,
            MoveMode::Targeting(TargetMove {
                target_group_id: args.ptr_reset_group as i32,
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
            args.ptrpos_id,
            ItemType::Counter,
            0.0,
            Op::Set,
            None,
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
    Ok(HandlerData::from_objects(vec![
        toggle_trigger(&top_cfg, args.write_group, !toggle_read),
        toggle_trigger(&bottom_cfg, args.read_group, toggle_read),
    ]))
}

fn mwrite(args: HandlerArgs) -> HandlerReturn {
    mem_mode(args, false)
}
fn mread(args: HandlerArgs) -> HandlerReturn {
    mem_mode(args, true)
}

fn display(args: HandlerArgs) -> HandlerReturn {
    let (id, t) = get_item_spec(&args.args[0]).unwrap();
    let cfg = GDObjConfig::new()
        .pos(-75.0, 75.0 + 30.0 * args.displayed_items as f64)
        .scale(0.5, 0.5);

    let obj = counter_object(
        &cfg,
        id,
        match t {
            ItemType::Timer => true,
            _ => false,
        },
        ItemAlign::Center,
        false,
        match t {
            ItemType::Attempts => Some(CounterMode::Attempts),
            ItemType::MainTime => Some(CounterMode::MainTime),
            ItemType::Points => Some(CounterMode::Points),
            _ => None,
        },
    );

    Ok(HandlerData::from_objects(vec![obj])
        .skip_spaces(0)
        .added_item_display())
}

pub fn ioblock(args: HandlerArgs) -> HandlerReturn {
    let spawn_group = args.args[0].to_group_id().unwrap();
    let position = args.args[1].to_int().unwrap();
    let msg = args.args[2].to_string().unwrap();
    let cfg = GDObjConfig::new().pos(75.0 + position as f64 * 30.0, 75.0);
    let text_cfg = cfg.clone().scale(0.25, 0.25);
    let spawn_cfg = cfg.clone().touchable(true);

    Ok(HandlerData::from_objects(vec![
        default_block(&cfg),
        spawn_trigger(
            &spawn_cfg,
            spawn_group as i32,
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

// fn malloc(args: HandlerArgs) -> HandlerReturn {
//     // TODO: do stuff
//     // TODO 2: return the groups of: ptr collblock, reset block
// }
