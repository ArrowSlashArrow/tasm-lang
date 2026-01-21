use gdlib::gdobj::{GDObjConfig, GDObject, misc::default_block};

use crate::core::{HandlerData, HandlerReturn, TasmPrimitive, TasmValue, TasmValueType};

pub const INSTR_SPEC: &[(
    &'static str,                                               // ident
    bool,                                                       // exclusive to _init
    &[(&[TasmValueType], fn(Vec<TasmValue>) -> HandlerReturn)], // handlers
)] = &[(
    "MALLOC",
    true,
    &[(&[TasmValueType::List(TasmPrimitive::Int)], todo)],
)];

fn todo(args: Vec<TasmValue>) -> HandlerReturn {
    //
    Ok(HandlerData::object(default_block(&GDObjConfig::default())))
}
