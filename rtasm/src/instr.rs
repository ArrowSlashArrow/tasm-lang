use gdlib::gdobj::{
    GDObjConfig, GDObject,
    misc::default_block,
    triggers::{ItemType, Op, RoundMode, SignMode, item_edit},
};

use crate::core::{HandlerData, HandlerFn, HandlerReturn, TasmPrimitive, TasmValue, TasmValueType};

// pub type ArithmeticInstrHandler = fn(Vec<TasmValue>, &GDObjConfig, Op) -> GDObject;

pub const INSTR_SPEC: &[(
    &'static str,                     // ident
    bool,                             // exclusive to _init
    &[(&[TasmValueType], HandlerFn)], // handlers
)] = &[
    (
        "MALLOC",
        true,
        &[(&[TasmValueType::List(TasmPrimitive::Int)], todo)],
    ),
    ("NOP", false, &[(&[], nop)]),
    (
        "WAIT",
        false,
        &[(&[TasmValueType::Primitive(TasmPrimitive::Int)], wait)],
    ),
    (
        "ADD",
        false,
        &[
            (
                &[
                    TasmValueType::Primitive(TasmPrimitive::Item),
                    TasmValueType::Primitive(TasmPrimitive::Item),
                ],
                add_2items,
            ),
            (
                &[
                    TasmValueType::Primitive(TasmPrimitive::Item),
                    TasmValueType::Primitive(TasmPrimitive::Number),
                ],
                add_item_num,
            ),
        ],
    ),
    (
        "SUB",
        false,
        &[
            (
                &[
                    TasmValueType::Primitive(TasmPrimitive::Item),
                    TasmValueType::Primitive(TasmPrimitive::Item),
                ],
                sub_2items,
            ),
            (
                &[
                    TasmValueType::Primitive(TasmPrimitive::Item),
                    TasmValueType::Primitive(TasmPrimitive::Number),
                ],
                sub_item_num,
            ),
        ],
    ),
    (
        "MUL",
        false,
        &[
            (
                &[
                    TasmValueType::Primitive(TasmPrimitive::Item),
                    TasmValueType::Primitive(TasmPrimitive::Item),
                ],
                mul_2items,
            ),
            (
                &[
                    TasmValueType::Primitive(TasmPrimitive::Item),
                    TasmValueType::Primitive(TasmPrimitive::Number),
                ],
                mul_item_num,
            ),
        ],
    ),
    (
        "DIV",
        false,
        &[
            (
                &[
                    TasmValueType::Primitive(TasmPrimitive::Item),
                    TasmValueType::Primitive(TasmPrimitive::Item),
                ],
                div_2items,
            ),
            (
                &[
                    TasmValueType::Primitive(TasmPrimitive::Item),
                    TasmValueType::Primitive(TasmPrimitive::Number),
                ],
                div_item_num,
            ),
        ],
    ),
];

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

fn nop(_args: Vec<TasmValue>, _cfg: &GDObjConfig) -> HandlerReturn {
    // skip no-op space
    Ok(HandlerData::default().skip_spaces(1))
}

fn wait(args: Vec<TasmValue>, _cfg: &GDObjConfig) -> HandlerReturn {
    // skip specified amount of spaces

    Ok(HandlerData::default().skip_spaces(args[0].to_int().unwrap()))
}

fn _arithmetic_2items(args: Vec<TasmValue>, cfg: &GDObjConfig, op: Op) -> GDObject {
    let (res_id, res_t) = get_item_spec(&args[0]).unwrap();
    let (op_id, op_t) = get_item_spec(&args[1]).unwrap();
    item_edit(
        &cfg,
        Some((op_id as i32, op_t)),
        None,
        res_id,
        res_t,
        1.0,
        Op::Set,
        None,
        Some(op),
        RoundMode::None,
        RoundMode::None,
        SignMode::None,
        SignMode::None,
    )
}

fn _arithmetic_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig, op: Op) -> GDObject {
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
        RoundMode::None,
        SignMode::None,
        SignMode::None,
    )
}

fn add_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Add,
    )]))
}

fn sub_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Sub,
    )]))
}
fn mul_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Mul,
    )]))
}
fn div_2items(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Div,
    )]))
}

fn add_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Add,
    )]))
}

fn sub_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Sub,
    )]))
}
fn mul_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Mul,
    )]))
}
fn div_item_num(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_item_num(
        args,
        cfg,
        Op::Div,
    )]))
}

// fn arithmetic(
//     args: Vec<TasmValue>,
//     cfg: &GDObjConfig,
//     inner: ArithmeticInstrHandler,
//     op: Op,
// ) -> GDObject {
//     inner(args, cfg, op)
// }
