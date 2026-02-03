use gdlib::gdobj::{
    GDObjConfig, GDObject,
    misc::default_block,
    triggers::{ItemType, Op, RoundMode, SignMode, item_edit},
};

use crate::core::{HandlerData, HandlerFn, HandlerReturn, TasmPrimitive, TasmValue, TasmValueType};

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
    ("MALLOC", true, &[argset!((Int) => todo)]),
    ("FMALLOC", true, &[argset!((Int) => todo)]),
    ("INITMEM", true, &[argset!([Number] => todo)]),
    ("NOP", false, &[argset!(() => nop)]),
    ("WAIT", false, &[argset!((Int) => wait)]),
    (
        "ADD",
        false,
        &[
            argset!((Item, Item) => add_2items),
            argset!((Item, Number) => add_item_num),
            argset!((Item, Item, Item) => add_3items),
        ],
    ),
    (
        "SUB",
        false,
        &[
            argset!((Item, Item) => sub_2items),
            argset!((Item, Number) => sub_item_num),
            argset!((Item, Item, Item) => sub_3items),
        ],
    ),
    (
        "MUL",
        false,
        &[
            argset!((Item, Item) => mul_2items),
            argset!((Item, Number) => mul_item_num),
            argset!((Item, Item, Item) => mul_3items),
            argset!((Item, Item, Number) => mul_2items_num),
        ],
    ),
    (
        "DIV",
        false,
        &[
            argset!((Item, Item) => div_2items),
            argset!((Item, Number) => div_item_num),
            argset!((Item, Item, Item) => div_3items),
            argset!((Item, Item, Number) => div_2items_num),
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
            argset!((Group, Item, Item) => todo),
            argset!((Group, Item, Number) => todo),
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

fn todo(_args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![default_block(cfg)]))
}

/* WAIT */

fn nop(_args: Vec<TasmValue>, _cfg: &GDObjConfig) -> HandlerReturn {
    // skip no-op space
    Ok(HandlerData::default().skip_spaces(1))
}

fn wait(args: Vec<TasmValue>, _cfg: &GDObjConfig) -> HandlerReturn {
    // skip specified amount of spaces
    Ok(HandlerData::default().skip_spaces(args[0].to_int().unwrap()))
}

/* ARITHMETIC */

fn _arithmetic_2items(
    args: Vec<TasmValue>,
    cfg: &GDObjConfig,
    op: Op,
    round_res: bool,
) -> GDObject {
    let (res_id, res_t) = get_item_spec(&args[0]).unwrap();
    let (op_id, op_t) = get_item_spec(&args[1]).unwrap();
    item_edit(
        &cfg,
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
    )
}

fn _arithmetic_3items(
    args: Vec<TasmValue>,
    cfg: &GDObjConfig,
    op: Op,
    round_res: bool,
) -> GDObject {
    let (res_id, res_t) = get_item_spec(&args[0]).unwrap();
    let (op1_id, op1_t) = get_item_spec(&args[1]).unwrap();
    let (op2_id, op2_t) = get_item_spec(&args[2]).unwrap();
    item_edit(
        &cfg,
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
    )
}

fn _arithmetic_item_num(
    args: Vec<TasmValue>,
    cfg: &GDObjConfig,
    op: Op,
    round_res: bool,
) -> GDObject {
    let (res_id, res_t) = get_item_spec(&args[0]).unwrap();
    // second arg should always be a number
    let modifier = args[1].to_float().unwrap();
    item_edit(
        &cfg,
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
    )
}

fn _arithmetic_2items_num(
    args: Vec<TasmValue>,
    cfg: &GDObjConfig,
    op: Op,
    round_res: bool,
) -> GDObject {
    let (res_id, res_t) = get_item_spec(&args[0]).unwrap();
    let (op1_id, op1_t) = get_item_spec(&args[1]).unwrap();
    let mult = args[2].to_float().unwrap();
    item_edit(
        &cfg,
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
    )
}

fn add_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Add,
        false,
    )]))
}
fn sub_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Sub,
        false,
    )]))
}
fn mul_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Mul,
        false,
    )]))
}
fn div_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Div,
        false,
    )]))
}
fn fldiv_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Div,
        true,
    )]))
}

fn add_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Add,
        false,
    )]))
}
fn sub_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Sub,
        false,
    )]))
}
fn mul_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Mul,
        false,
    )]))
}
fn div_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Div,
        false,
    )]))
}
fn fldiv_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Div,
        true,
    )]))
}

fn add_3items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_3items(
        args,
        cfg,
        Op::Add,
        false,
    )]))
}
fn sub_3items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_3items(
        args,
        cfg,
        Op::Sub,
        false,
    )]))
}
fn mul_3items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_3items(
        args,
        cfg,
        Op::Mul,
        false,
    )]))
}
fn div_3items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_3items(
        args,
        cfg,
        Op::Div,
        false,
    )]))
}
fn fldiv_3items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_3items(
        args,
        cfg,
        Op::Div,
        true,
    )]))
}

fn mul_2items_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items_num(
        args,
        cfg,
        Op::Mul,
        false,
    )]))
}
fn div_2items_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items_num(
        args,
        cfg,
        Op::Div,
        false,
    )]))
}
fn fldiv_2items_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items_num(
        args,
        cfg,
        Op::Div,
        true,
    )]))
}
