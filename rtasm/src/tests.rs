use crate::{
    core::{TasmValue, TasmValueType},
    lexer::fits_arg_sig,
};

use super::*;

#[test]
fn int_detection() {
    assert!(fits_arg_sig(
        &vec![TasmValue::Number(1.0), TasmValue::Number(1.1)],
        &[
            TasmValueType::Primitive(core::TasmPrimitive::Int),
            TasmValueType::Primitive(core::TasmPrimitive::Number),
        ],
    ))
}

#[test]
fn no_int_detection() {
    assert!(!fits_arg_sig(
        &vec![TasmValue::Number(1.1), TasmValue::Number(1.1)],
        &[
            TasmValueType::Primitive(core::TasmPrimitive::Int),
            TasmValueType::Primitive(core::TasmPrimitive::Number),
        ],
    ))
}
