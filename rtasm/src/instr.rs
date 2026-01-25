use gdlib::gdobj::{GDObjConfig, GDObject, misc::default_block};

use crate::core::{HandlerData, HandlerFn, HandlerReturn, TasmPrimitive, TasmValue, TasmValueType};

pub const INSTR_SPEC: &[(
    &'static str,                     // ident
    bool,                             // exclusive to _init
    &[(&[TasmValueType], HandlerFn)], // handlers
)] = &[(
    "MALLOC",
    true,
    &[(&[TasmValueType::List(TasmPrimitive::Int)], todo)],
)];

fn todo(args: Vec<TasmValue>, cfg: &GDObjConfig) -> HandlerReturn {
    //
    Ok(HandlerData::from_objects(vec![default_block(cfg)]))
}
