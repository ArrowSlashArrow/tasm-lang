use paste::paste;
use std::time::Instant;

use crate::core::structs::{TasmPrimitive, TasmValue, TasmValueType, fits_arg_signature};

use super::*;

macro_rules! tasm_test {
    // successful compile
    ($file:literal, true) => {
        paste! {
            #[test]
            fn [<compile_success _ $file>]() {
                let mut res = lexer::parse_file(
                    fs::read_to_string(format!("../tests/{}.tasm", $file)).unwrap(),
                    format!("testfile {}", $file),
                    9999,
                    0,
                    true,
                    true,
                    false
                ).unwrap();
                assert!(res.handle_routines(&String::new()).is_ok())
            }
        }
    };
    // fail in lexing stage
    ($file:literal, false) => {
        paste! {
            #[test]
            fn [<fileparse_fail _ $file>]() {
                assert!(lexer::parse_file(
                    fs::read_to_string(format!("../tests/{}.tasm", $file)).unwrap(),
                    format!("testfile {}", $file),
                    9999,
                    0,
                    true,
                    true,
                    false
                ).is_err())
            }
        }
    };
    // fail in translation stage
    ($file:literal, false, compile) => {
        paste! {
            #[test]
            fn [<translate_fail _ $file>]() {
                let mut res = lexer::parse_file(
                    fs::read_to_string(format!("../tests/{}.tasm", $file)).unwrap(),
                    format!("testfile {}", $file),
                    9999,
                    0,
                    true,
                    true,
                    false
                ).unwrap();
                assert!(res.handle_routines(&String::new()).is_err())
            }
        }
    };
    // file in the `example_programs` directory
    ($file:literal, example) => {
        paste! {
            #[test]
            fn [<example _ $file>]() {
                let mut res = lexer::parse_file(
                    fs::read_to_string(format!("../example_programs/{}.tasm", $file)).unwrap(),
                    format!("testfile {}", $file),
                    9999,
                    0,
                    true,
                    true,
                    false
                ).unwrap();
                assert!(res.handle_routines(&String::new()).is_ok())
            }
        }
    };

    // file in the `example_programs` directory without an entry point
    ($file:literal, example_no_entry_point) => {
        paste! {
            #[test]
            fn [<example _ $file>]() {
                let mut res = lexer::parse_file(
                    fs::read_to_string(format!("../example_programs/{}.tasm", $file)).unwrap(),
                    format!("testfile {}", $file),
                    9999,
                    0,
                    true,
                    true,
                    true
                ).unwrap();
                assert!(res.handle_routines(&String::new()).is_ok())
            }
        }
    };

    // tests compiler-defined implementations located in `tests/compdef_{ident}.tasm`
    ($file:literal, compdef) => {
        paste! {
            #[test]
            fn [<compdef _ $file>]() {
                let mut res = lexer::parse_file(
                    fs::read_to_string(format!("../tests/compdef_{}.tasm", $file)).unwrap(),
                    format!("testfile {}", $file),
                    9999,
                    0,
                    true,
                    true,
                    true // no entry point, since the routine should be named the same as the ident
                ).unwrap();
                assert!(res.handle_routines(&String::new()).is_ok())
            }
        }
    };
}

tasm_test!("fetch", example_no_entry_point);
tasm_test!("fib_in_memory", example);
tasm_test!("incrementer", example);
tasm_test!("is_c1_prime", example);
tasm_test!("pointer_test", example);
tasm_test!("pointer_test1", example);
tasm_test!("proc_control", example);
tasm_test!("project_euler_1", example);
tasm_test!("project_euler_2", example);
tasm_test!("project_euler_6", example);
tasm_test!("rng", example);
tasm_test!("aliases", true);
tasm_test!("all_instructions", true);
tasm_test!("bad_args", false);
tasm_test!("bad_assignment", false, compile);
tasm_test!("bad_instruction", false);
tasm_test!("bad_token", false);
tasm_test!("concurrent", true);
tasm_test!("correct", true);
tasm_test!("empty", true);
tasm_test!("flags", true);
tasm_test!("init_rtn_mem", false);
tasm_test!("init_spawn", false);
tasm_test!("lowercase", true);
tasm_test!("multiple_mem", false, compile);
tasm_test!("multiple_routines", false);
tasm_test!("negative_ids", false);
tasm_test!("no_entry_point", false);
tasm_test!("no_memory", false, compile);
tasm_test!("recursive", true);
tasm_test!("tab_spacing", true);
tasm_test!("timer_not_counter", false);
tasm_test!("timerops", true);
tasm_test!("trailing_comma", false);
tasm_test!("values", true);
// compdef: internal compiler-defined implementation
tasm_test!("swap", compdef);
tasm_test!("min", compdef);
tasm_test!("max", compdef);

#[test]
fn int_detection() {
    assert!(fits_arg_signature(
        &vec![TasmValue::Number(1.0), TasmValue::Number(1.1)],
        &[
            TasmValueType::Primitive(TasmPrimitive::Int),
            TasmValueType::Primitive(TasmPrimitive::Number),
        ],
    ))
}

#[test]
fn no_int_detection() {
    assert!(!fits_arg_signature(
        &vec![TasmValue::Number(1.1), TasmValue::Number(1.1)],
        &[
            TasmValueType::Primitive(TasmPrimitive::Int),
            TasmValueType::Primitive(TasmPrimitive::Number),
        ],
    ))
}

#[test]
fn parse_tasm() -> anyhow::Result<()> {
    let file = fs::read_to_string("../programs/nuclear_reactor.tasm")?;
    let mut parse_start = Instant::now();
    let mut tasm = lexer::parse_file(
        file,
        format!("../programs/nuclear_reactor.tasm"),
        9999,
        0,
        true,
        true,
        false,
    )
    .unwrap();

    println!(
        "Parse time: {:.3}ms",
        parse_start.elapsed().as_micros() as f64 / 1000.0
    );

    parse_start = Instant::now();
    let _level = tasm.handle_routines(&"test level".into()).unwrap();
    println!(
        "Serialise time: {:.3}ms",
        parse_start.elapsed().as_micros() as f64 / 1000.0
    );

    // level.export_to_gmd("test.gmd")?;
    Ok(())
}
