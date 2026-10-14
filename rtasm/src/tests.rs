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
tasm_test!("coin_counter", example);
tasm_test!("collatz", example);
tasm_test!("incrementer", example);
tasm_test!("is_c1_prime", example);
tasm_test!("list_search", example);
// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// tasm_test!("pointer_test", example);
// this benchmark is no longer valid because the file uses deprecated syntax which is no longer supported by the compiler
// tasm_test!("pointer_test1", example);
tasm_test!("memory_basics", example);
tasm_test!("proc_control", example);
tasm_test!("project_euler_1", example);
tasm_test!("project_euler_2", example);
tasm_test!("project_euler_6", example);
tasm_test!("rng", example);
tasm_test!("stdlib_modulo", example);
tasm_test!("stopwatch", example);
tasm_test!("temperature", example);
tasm_test!("two_workers", example);
tasm_test!("use_shapes", example);
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
tasm_test!("math", stdlib);
tasm_test!("compare", stdlib);
tasm_test!("bits", stdlib);
// sha256 of a string, implemented with one counter per bit
tasm_test!("sha256", true);

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

/// Regression test for helper group allocation (unresolved_issues.md, issue 1.1).
///
/// Compiles `tests/aux_groups.tasm`, which only uses non-instant compares and randoms.
/// Every group targeted by an Item Compare or Random trigger is a helper group, so it must:
/// * not be the group of any routine, and
/// * contain exactly one object: the spawn trigger that the helper group exists for.
#[test]
fn aux_groups_are_unique() {
    use gdlib::cclocallevels::gdobj::ids::{
        objects::{ITEM_COMPARE_TRIGGER, RANDOM_TRIGGER, SPAWN_TRIGGER},
        properties::{TARGET_ITEM, TARGET_ITEM_2},
    };
    use gdlib::cclocallevels::gdobj::structs::GDValue;
    use std::collections::HashSet;

    let (mut res, group) = crate::parse_main(&crate::Args::test_args(
        "../tests/aux_groups.tasm".into(),
        true,
    ))
    .unwrap();
    let level = match res.handle_routines_inner("", group, false, false) {
        Ok(level) => level,
        Err(e) => {
            print_errors(e, "errors");
            panic!()
        }
    };
    let objects = level.get_decrypted_data().unwrap().objects;

    let routine_groups: HashSet<i16> = res
        .routines
        .iter()
        .map(|r| r.group)
        .filter(|&g| g != 0)
        .collect();

    // every group that a compare or random trigger spawns
    let mut helper_groups = vec![];
    for obj in objects
        .iter()
        .filter(|o| o.id == ITEM_COMPARE_TRIGGER || o.id == RANDOM_TRIGGER)
    {
        for (key, value) in &obj.properties {
            if let (TARGET_ITEM | TARGET_ITEM_2, GDValue::Group(g)) = (*key, value)
                && *g != 0
            {
                helper_groups.push(*g);
            }
        }
    }
    assert!(
        !helper_groups.is_empty(),
        "no compare or random triggers found"
    );

    let mut seen = HashSet::new();
    for g in helper_groups {
        assert!(
            !routine_groups.contains(&g),
            "helper group {g} is also the group of a routine"
        );
        assert!(
            seen.insert(g),
            "helper group {g} is spawned by more than one compare/random trigger"
        );

        let members = objects
            .iter()
            .filter(|o| o.config.groups.iter().any(|grp| grp.id() == g))
            .collect::<Vec<_>>();
        assert_eq!(
            members.len(),
            1,
            "helper group {g} contains {} objects instead of 1",
            members.len()
        );
        assert_eq!(
            members[0].id, SPAWN_TRIGGER,
            "helper group {g} does not contain a spawn trigger"
        );
    }
}
