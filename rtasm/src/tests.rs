use std::time::Instant;

use crate::core::{TasmValue, TasmValueType, fits_arg_signature};

use super::*;

#[test]
fn int_detection() {
    assert!(fits_arg_signature(
        &vec![
            TasmValue::Number(1.0),
            // TasmValue::Number(1.1)
        ],
        &[
            TasmValueType::Primitive(core::TasmPrimitive::Int),
            // TasmValueType::Primitive(core::TasmPrimitive::Number),
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
    let mut tasm = match lexer::parse_file(file, 9999) {
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
