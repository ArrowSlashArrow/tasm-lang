use gdlib::gdobj::{GDObjConfig, GDObject, misc::default_block};

use crate::core::{TasmValue, TasmValueType};

pub const INSTR_SPEC: &[(
    &'static str,
    &'static str,
    &[(&[TasmValueType], fn(Vec<TasmValue>) -> GDObject)],
)] = &[(
    "MALLOC",
    "_init",
    &[(&[TasmValueType::Int], malloc_handler)],
)];

fn malloc_handler(args: Vec<TasmValue>) -> GDObject {
    // todo
    default_block(&GDObjConfig::default())
}
