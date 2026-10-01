use gdlib::cclocallevels::gdobj::{
    GDObject, ItemEditTrigger, ObjectProperties,
    ids::{objects::ITEM_EDIT_TRIGGER, properties::*},
    structs::{GDObjPropType, GDValue, Item, ItemType, Op, RoundMode, SignMode},
};
use paste::paste;

use crate::core::structs::{TasmPrimitive, TasmValue, fits_arg_signature};

use super::*;

macro_rules! tasm_test {
    // successful compile
    ($file:literal, true) => {
        paste! {
            #[test]
            fn [<compile_success _ $file>]() {
                let (mut res, group) = match crate::parse_main(&crate::Args::test_args(
                    format!("../tests/{}.tasm", $file),
                    true
                )) {
                    Ok(v) => v,
                    Err(e) => {
                        panic!("{e}");
                    }
                };

                match res.handle_routines_inner("", group, false, false) {
                    Ok(_) => return,
                    Err(e) => {
                        print_errors(e, "errors");
                        panic!()
                    }
                }
            }
        }
    };
    // fail in lexing stage
    ($file:literal, false) => {
        paste! {
            #[test]
            fn [<fileparse_fail _ $file>]() {
                assert!(crate::parse_main(&crate::Args::test_args(
                    format!("../tests/{}.tasm", $file),
                    true
                )).is_err());
            }
        }
    };
    // fail in translation stage
    ($file:literal, false, compile) => {
        paste! {
            #[test]
            fn [<translate_fail _ $file>]() {
                let (mut res, group) = crate::parse_main(&crate::Args::test_args(
                    format!("../tests/{}.tasm", $file),
                    true
                ))
                .unwrap();
                assert!(res.handle_routines_inner("", group, false, false).is_err());
            }
        }
    };
    // file in the `example_programs` directory
    ($file:literal, example) => {
        paste! {
            #[test]
            fn [<example _ $file>]() {
                let (mut res, group) = crate::parse_main(&crate::Args::test_args(
                    format!("../example_programs/{}.tasm", $file),
                    true
                ))
                .unwrap();
                match res.handle_routines_inner("", group, false, false) {
                    Ok(_) => return,
                    Err(e) => {
                        print_errors(e, "errors");
                        panic!()
                    }
                }
            }
        }
    };

    // // file in the `example_programs` directory without an entry point
    // ($file:literal, example_no_entry_point) => {
    //     paste! {
    //         #[test]
    //         fn [<example _ $file>]() {
    //             let (mut res, group) = crate::parse_main(&crate::Args::test_args(
    //                 format!("../example_programs/{}.tasm", $file),
    //                 false
    //             ))
    //             .unwrap();
    //             match res.handle_routines_inner("", group, false, false) {
    //                 Ok(_) => return,
    //                 Err(e) => {
    //                     print_errors(e, "errors");
    //                     panic!()
    //                 }
    //             }
    //         }
    //     }
    // };

    // tests compiler-defined implementations located in `tests/compdef_{ident}.tasm`
    // todo: move these to stdlib
    ($file:literal, compdef) => {
        paste! {
            #[test]
            fn [<compdef _ $file>]() {
                let (mut res, group) = crate::parse_main(&crate::Args::test_args(
                    format!("../tests/compdef_{}.tasm", $file),
                    false,
                ))
                .unwrap();
                res.handle_routines_inner("", group, false, false).unwrap();
            }
        }
    };

    // tests stdlib stuff located in `stdlib/{ident}.tasm`. these should *always* work.
    ($file:literal, stdlib) => {
        paste! {
            #[test]
            fn [<stdlib _ $file>]() {
                let (mut res, group) = crate::parse_main(&crate::Args::test_args(
                    format!("../stdlib/{}.tasm", $file),
                    false, // no entry point on stdlib files
                ))
                .unwrap();
                res.handle_routines_inner("", group, false, false).unwrap();
            }
        }
    };
}

// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// tasm_test!("fetch", example_no_entry_point);
// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// tasm_test!("fib_in_memory", example);
tasm_test!("incrementer", example);
tasm_test!("is_c1_prime", example);
tasm_test!("list_search", example);
// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// tasm_test!("pointer_test", example);
// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// tasm_test!("pointer_test1", example);
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
tasm_test!("circular_import", false);
tasm_test!("concurrent", true);
tasm_test!("correct", true);
tasm_test!("diamond_dependency", true);
tasm_test!("division", true);
tasm_test!("empty", false);
tasm_test!("flags", true);
tasm_test!("hex_identifiers", true);
tasm_test!("init_rtn_mem", false);
tasm_test!("init_spawn", false);
tasm_test!("link_test", true);
tasm_test!("lowercase", true);
tasm_test!("memory", true);
// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// tasm_test!("multiple_mem", false, compile);
tasm_test!("multiple_routines", false);
tasm_test!("negative_ids", false);
tasm_test!("no_entry_point", false);
// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// tasm_test!("no_memory", false, compile);
tasm_test!("recursive", true);
tasm_test!("remap_alias", true);
tasm_test!("tab_spacing", true);
tasm_test!("timer_not_counter", false);
tasm_test!("timerops", true);
tasm_test!("trailing_comma", false);
tasm_test!("values", true);
tasm_test!("vec_test", true);
// compdef: internal compiler-defined implementation
tasm_test!("swap", compdef);
tasm_test!("min", compdef);
tasm_test!("max", compdef);
// stdlib
tasm_test!("mem_8bit", stdlib);
tasm_test!("mem_14bit", stdlib);

impl Args {
    pub fn test_args(infile: String, has_entry: bool) -> Self {
        Self {
            infile: Some(infile),
            verbose_logs: true,
            dependencies: true,
            linker_output: true,
            no_entry_point: !has_entry,
            ..Default::default()
        }
    }
}

#[test]
fn int_detection() {
    assert!(fits_arg_signature(
        &[TasmValue::Number(1.0), TasmValue::Number(1.1)],
        &[TasmPrimitive::Int, TasmPrimitive::Number,],
    ))
}

#[test]
fn no_int_detection() {
    assert!(!fits_arg_signature(
        &[TasmValue::Number(1.1), TasmValue::Number(1.1)],
        &[TasmPrimitive::Int, TasmPrimitive::Number,],
    ))
}

// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// #[test]
// fn parse_tasm() -> anyhow::Result<()> {
//     let mut parse_start = Instant::now();
//     let (mut res, group) = crate::parse_main(&crate::Args::test_args(
//         "../programs/nuclear_reactor.tasm".into(),
//         true,
//     ))
//     .unwrap();

//     println!(
//         "Parse time: {:.3}ms",
//         parse_start.elapsed().as_micros() as f64 / 1000.0
//     );

//     parse_start = Instant::now();
//     let _level = match res.handle_routines_inner("", group, false, false) {
//         Ok(m) => m,
//         Err(e) => {
//             print_errors(e, "errors");
//             panic!()
//         }
//     };
//     println!(
//         "Serialise time: {:.3}ms",
//         parse_start.elapsed().as_micros() as f64 / 1000.0
//     );

//     // level.export_to_gmd("test.gmd")?;
//     Ok(())
// }

// temporary test for
#[ignore]
#[test]
fn list_instructions() {
    let objects = &mut [
        "1,3619,2,943.407,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,1,479,1,480,3,481,1,482,3",
        "1,3619,2,966.7,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,990.963,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,6,480,4,481,1,482,3",
        "1,3619,2,919.142,3,384.038,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,3,479,1,481,1,482,3",
        "1,3619,2,1014.25,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,2,481,1,482,3",
        "1,3619,2,1062.76,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1086.04,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,120,480,4,481,1,482,3",
        "1,3619,2,1110.28,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,1,481,1,482,3",
        "1,3619,2,1038.51,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,1157.83,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1229.64,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,5040,480,4,481,1,482,3",
        "1,3619,2,1252.91,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,2,481,1,482,3",
        "1,3619,2,1133.57,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,1181.12,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1300.44,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1348.93,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,362880,480,4,481,1,482,3",
        "1,3619,2,1372.22,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,1,481,1,482,3",
        "1,3619,2,1277.17,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,1324.7,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1419.78,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1491.58,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,3.99168e+06,480,4,481,1,482,3",
        "1,3619,2,1539.09,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,2,481,1,482,3",
        "1,3619,2,1396.48,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,1444.04,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1467.33,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1563.35,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,476,2,478,2,51,3,479,1,480,3,481,1,482,3",
        "1,3619,2,1515.84,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,10,480,4,481,1,482,3",
        "1,3619,2,824.091,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,35643.2,481,1,482,3",
        "1,3619,2,847.361,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,1e+07,480,4,481,1,482,3",
        "1,3619,2,871.616,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,1,480,1,481,1,482,3",
        "1,3619,2,895.876,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,57.5,480,4,481,1,482,3",
        "1,3619,2,1205.38,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,680.469,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,1,479,90,480,2,481,1,482,3",
        "1,3619,2,704.731,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,1,476,2,478,2,51,2,479,360,481,1,482,4,485,2",
        "1,3619,2,728.985,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,360,481,1,482,3",
        "1,3619,2,752.284,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,1,95,3,476,2,477,2,478,2,51,3,479,1,481,2,482,3",
        "1,3619,2,776.537,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,3,479,180,480,2,481,2,482,3,579,1",
        "1,3619,2,776.537,3,399.592,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,2,479,90,481,1,482,3",
        "1,3619,2,799.829,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,3,95,2,476,2,477,2,478,2,51,1,479,1,481,2,482,3",
        "1,3619,2,657.199,3,399.592,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,5,476,2,478,2,51,4,479,1,481,1,482,3",
        "1,3619,2,680.469,3,399.592,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,4,479,90,480,1,481,1,482,3",
        "1,3619,2,1872.88,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,1,479,1,480,3,481,1,482,3",
        "1,3619,2,1897.14,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,1920.39,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,6,480,4,481,1,482,3",
        "1,3619,2,1849.58,3,384.038,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,3,479,1,481,1,482,3",
        "1,3619,2,1944.65,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,2,481,1,482,3",
        "1,3619,2,1992.17,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,2016.43,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,120,480,4,481,1,482,3",
        "1,3619,2,2039.75,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,1,481,1,482,3",
        "1,3619,2,1968.9,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,2087.28,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,2159.04,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,5040,480,4,481,1,482,3",
        "1,3619,2,2183.3,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,2,481,1,482,3",
        "1,3619,2,2064.01,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,2111.53,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,2230.86,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,2278.4,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,362880,480,4,481,1,482,3",
        "1,3619,2,2302.66,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,1,481,1,482,3",
        "1,3619,2,2206.59,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,2254.15,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,2350.17,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,2421.02,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,3.99168e+06,480,4,481,1,482,3",
        "1,3619,2,2469.53,3,368.401,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,1,480,2,481,1,482,3",
        "1,3619,2,2325.91,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,481,3,482,3",
        "1,3619,2,2373.46,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,2,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,2397.71,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,95,1,476,2,477,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,2492.8,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,476,2,478,2,51,3,479,1,480,3,481,1,482,3",
        "1,3619,2,2445.25,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,10,480,4,481,1,482,3",
        "1,3619,2,1753.52,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,35643.2,481,1,482,3",
        "1,3619,2,1777.78,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,1e+07,480,4,481,1,482,3",
        "1,3619,2,1801.07,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,1,480,1,481,1,482,3",
        "1,3619,2,1825.33,3,399.592,108,2,57,1,155,1,62,1,87,1,36,1,478,2,51,2,479,57.5,480,4,481,1,482,3",
        "1,3619,2,2135.79,3,384.038,108,2,57,1,155,1,62,1,87,1,36,1,80,1,476,2,478,2,51,2,479,1,480,3,481,3,482,3",
        "1,3619,2,1610.91,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,1,479,90,480,2,481,1,482,3",
        "1,3619,2,1634.2,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,1,476,2,478,2,51,2,479,360,481,1,482,4,485,2",
        "1,3619,2,1658.47,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,2,476,2,478,2,51,3,479,360,481,1,482,3",
        "1,3619,2,1682.71,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,1,95,3,476,2,477,2,478,2,51,3,479,1,481,2,482,3",
        "1,3619,2,1705.96,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,3,479,180,480,2,481,2,482,3,579,1",
        "1,3619,2,1705.96,3,399.592,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,478,2,51,2,479,90,481,1,482,3",
        "1,3619,2,1730.22,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,3,95,2,476,2,477,2,478,2,51,1,479,1,481,2,482,3",
        "1,3619,2,1586.65,3,415.206,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,4,476,2,478,2,51,1,479,1,481,1,482,3",
        "1,3619,2,1610.91,3,384.038,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,3,476,2,478,2,51,4,479,1,481,1,482,3",
        "1,3619,2,657.199,3,304.794,108,2,57,1,155,1,25,2,24,5,62,1,87,1,36,1,80,5,476,2,478,2,51,1,479,1,481,1,482,3",
    ].map(|o| GDObject::parse_str(o));
    objects.sort_by(|a, b| a.config.pos.0.total_cmp(&b.config.pos.0));

    for obj in objects.iter() {
        let object = from_object(&obj).unwrap();
        let expr = print_expr(&object);
        // println!("{object:?}");
        println!("Object at {:?} : {expr}", obj.config.pos);
    }
}

fn item_str(i: Item) -> String {
    match i {
        Item::Attempts => "Attempts".into(),
        Item::Points => "Points".into(),
        Item::MainTime => "MainTime".into(),
        Item::Counter(c) => format!("C{c}"),
        Item::Timer(t) => format!("T{t}"),
    }
}

fn print_expr(trg: &ItemEditTrigger) -> String {
    let idop = op_to_str(trg.id_op);
    let mut id_result = match (trg.operand1, trg.operand2) {
        (Some(a), Some(b)) => format!("({} {idop} {})", item_str(a), item_str(b)),
        (Some(a), None) => item_str(a),
        (None, Some(b)) => item_str(b),
        (None, None) => String::new(),
    };
    let mod_str = trg.modifier.to_string();

    if id_result == String::new() {
        id_result = mod_str;
    } else if trg.modifier != 1.0 {
        id_result += if trg.multiply_mod { " * " } else { " / " };
        id_result += &mod_str[..];
    }

    match trg.id_rounding {
        RoundMode::None => {}
        r => id_result = format!("{r:?}({id_result})"),
    }
    match trg.id_sign {
        SignMode::None => {}
        s => id_result = format!("{s:?}({id_result})"),
    }

    match trg.assign_op {
        Op::Set => {}
        o => id_result = format!("{} {} {id_result}", item_str(trg.target), op_to_str(o)),
    }

    match trg.result_rounding {
        RoundMode::None => {}
        r => id_result = format!("{r:?}({id_result})"),
    }
    match trg.result_sign {
        SignMode::None => {}
        s => id_result = format!("{s:?}({id_result})"),
    }

    format!("{} = {id_result}", item_str(trg.target))
}

fn op_to_str(o: Op) -> &'static str {
    match o {
        Op::Set => "",
        Op::Add => "+",
        Op::Sub => "-",
        Op::Mul => "*",
        Op::Div => "/",
    }
}

fn from_object(obj: &GDObject) -> Option<ItemEditTrigger> {
    if obj.id != ITEM_EDIT_TRIGGER {
        return None;
    }

    // An operand is only present if its ID is non-zero.
    // (Assumes `Item::new(item_type, id)`; adjust to your actual `Item`.)
    let read_item = |id_prop, type_prop| -> Option<Item> {
        let id = match obj.get_property(id_prop) {
            Some(GDValue::Item(id)) if id != 0 => id,
            _ => return None,
        };
        let item_type = match obj.get_property(type_prop) {
            Some(GDValue::ItemType(t)) => t,
            _ => ItemType::default(),
        };
        Some(Item::from_id_type(id as i16, item_type))
    };

    let operand1 = read_item(INPUT_ITEM_1, FIRST_ITEM_TYPE);
    let operand2 = read_item(INPUT_ITEM_2, SECOND_ITEM_TYPE);

    // The target is required, so fall back to a default rather than returning None.
    let target = {
        let id = match obj.get_property(TARGET_ITEM) {
            Some(GDValue::Group(id)) => id,
            _ => 0,
        };
        let item_type = match obj.get_property(TARGET_ITEM_TYPE) {
            Some(GDValue::ItemType(t)) => t,
            _ => ItemType::default(),
        };
        Item::from_id_type(id as i16, item_type)
    };

    let modifier = match obj.get_property(MODIFIER) {
        Some(GDValue::Float(m)) => m,
        _ => 1.0,
    };

    // The in-game default for multiply_mod is "multiply".
    let multiply_mod = match obj.get_property(COMPARE_OPERATOR) {
        Some(GDValue::Bool(b)) => b,
        _ => true,
    };

    let assign_op = match obj.get_property(LEFT_OPERATOR) {
        Some(GDValue::ItemEditOperator(op)) => op,
        _ => Op::Set,
    };
    let id_op = match obj.get_property(RIGHT_OPERATOR) {
        Some(GDValue::ItemEditOperator(op)) => op,
        _ => Op::Add,
    };

    let id_rounding = match obj.get_property(LEFT_ROUND_MODE) {
        Some(GDValue::ItemEditRoundMode(m)) => m,
        _ => RoundMode::default(),
    };
    let result_rounding = match obj.get_property(RIGHT_ROUND_MODE) {
        Some(GDValue::ItemEditRoundMode(m)) => m,
        _ => RoundMode::default(),
    };

    let id_sign = match obj.get_property(LEFT_SIGN_MODE) {
        Some(GDValue::ItemEditSignMode(m)) => m,
        _ => SignMode::default(),
    };
    let result_sign = match obj.get_property(RIGHT_SIGN_MODE) {
        Some(GDValue::ItemEditSignMode(m)) => m,
        _ => SignMode::default(),
    };

    Some(ItemEditTrigger {
        operand1,
        operand2,
        target,
        modifier,
        assign_op,
        multiply_mod,
        id_op,
        id_rounding,
        result_rounding,
        id_sign,
        result_sign,
    })
}
