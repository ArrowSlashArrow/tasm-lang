use paste::paste;
use std::time::Instant;

use crate::core::{TasmValue, TasmValueType, fits_arg_signature};

use super::*;

macro_rules! tasm_test {
    // parser success
    ($file:literal, true) => {
        paste! {
            #[test]
            fn [<fileparse_pass _ $file>]() {
                assert!(lexer::parse_file(
                    fs::read_to_string(format!("../tests/{}.tasm", $file)).unwrap(),
                    9999,
                    0,
                    true
                ).is_ok())
            }
        }
    };
    // parser error handler
    ($file:literal, false) => {
        paste! {
            #[test]
            fn [<fileparse_fail _ $file>]() {
                assert!(lexer::parse_file(
                    fs::read_to_string(format!("../tests/{}.tasm", $file)).unwrap(),
                    9999,
                    0,
                    true
                ).is_err())
            }
        }
    };
    // compiler error handler
    ($file:literal, false, compile) => {
        paste! {
            #[test]
            fn [<fileparse_fail _ $file>]() {
                let mut res = lexer::parse_file(
                    fs::read_to_string(format!("../tests/{}.tasm", $file)).unwrap(),
                    9999,
                    0,
                    true
                ).unwrap();
                assert!(res.handle_routines(&String::new()).is_err())
            }
        }
    };
}

tasm_test!("all_instructions", true);
tasm_test!("bad_args", false);
tasm_test!("bad_instruction", false);
tasm_test!("bad_token", false);
tasm_test!("correct", true);
tasm_test!("empty", true);
tasm_test!("flags", false); // TODO: CHANGE TO TRUE WHEN FLAGS ARE ADDED
tasm_test!("init_rtn_mem", false);
tasm_test!("init_spawn", false);
tasm_test!("multiple_mem", false, compile);
tasm_test!("multiple_routines", false);
tasm_test!("no_entry_point", false);
tasm_test!("no_memory", false, compile);

#[test]
fn int_detection() {
    assert!(fits_arg_signature(
        &vec![TasmValue::Number(1.0), TasmValue::Number(1.1)],
        &[
            TasmValueType::Primitive(core::TasmPrimitive::Int),
            TasmValueType::Primitive(core::TasmPrimitive::Number),
        ],
    ))
}

#[test]
fn no_int_detection() {
    assert!(!fits_arg_signature(
        &vec![TasmValue::Number(1.1), TasmValue::Number(1.1)],
        &[
            TasmValueType::Primitive(core::TasmPrimitive::Int),
            TasmValueType::Primitive(core::TasmPrimitive::Number),
        ],
    ))
}

#[test]
fn parse_tasm() -> anyhow::Result<()> {
    let file = fs::read_to_string("../programs/nuclear_reactor.tasm")?;
    let mut parse_start = Instant::now();
    let mut tasm = match lexer::parse_file(file, 9999, 0, true) {
        Ok(t) => {
            println!("Parsed file with 0 errors.");
            t
        }
        Err(e) => {
            for err in e.iter() {
                println!("{err}");
            }
            println!("Parsed file with {} errors.", e.len());
            panic!("bad tasm")
        }
    };

    println!(
        "Parse time: {:.3}ms",
        parse_start.elapsed().as_micros() as f64 / 1000.0
    );

    parse_start = Instant::now();
    let level = tasm.handle_routines(&"test level".into()).unwrap();
    println!(
        "Serialise time: {:.3}ms",
        parse_start.elapsed().as_micros() as f64 / 1000.0
    );

    level.export_to_gmd("test.gmd")?;
    Ok(())
}
