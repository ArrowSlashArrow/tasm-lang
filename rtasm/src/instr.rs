use gdlib::gdobj::{
    GDObjConfig, GDObject,
    misc::default_block,
    triggers::{ItemType, Op, RoundMode, SignMode, item_edit},
};

use crate::core::{HandlerData, HandlerFn, HandlerReturn, TasmPrimitive, TasmValue, TasmValueType};

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
        &[(
            &[
                TasmValueType::Primitive(TasmPrimitive::Item),
                TasmValueType::Primitive(TasmPrimitive::Item),
            ],
            add,
        )],
    ),
    (
        "SUB",
        false,
        &[(
            &[
                TasmValueType::Primitive(TasmPrimitive::Item),
                TasmValueType::Primitive(TasmPrimitive::Item),
            ],
            sub,
        )],
    ),
    (
        "MUL",
        false,
        &[(
            &[
                TasmValueType::Primitive(TasmPrimitive::Item),
                TasmValueType::Primitive(TasmPrimitive::Item),
            ],
            mul,
        )],
    ),
    (
        "DIV",
        false,
        &[(
            &[
                TasmValueType::Primitive(TasmPrimitive::Item),
                TasmValueType::Primitive(TasmPrimitive::Item),
            ],
            div,
        )],
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

fn add(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Add,
    )]))
}

fn sub(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Sub,
    )]))
}
fn mul(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Mul,
    )]))
}
fn div(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![_arithmetic_2items(
        args,
        cfg,
        Op::Div,
    )]))
}
