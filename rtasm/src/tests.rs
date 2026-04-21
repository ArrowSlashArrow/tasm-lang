use gdlib::{
    gdlevel::Level,
    gdobj::{
        GDObjConfig, GDObject, GDValue, Group, Item,
        ids::{
            objects::TRIGGER_ITEM_COMPARE,
            properties::{
                COMPARE_OPERATOR, FIRST_ITEM_TYPE, INPUT_ITEM_1, INPUT_ITEM_2, LEFT_OPERATOR,
                LEFT_ROUND_MODE, LEFT_SIGN_MODE, MODIFIER, RIGHT_OPERATOR, RIGHT_ROUND_MODE,
                RIGHT_SIGN_MODE, SECOND_ITEM_TYPE, SECOND_MODIFIER, TARGET_ITEM, TARGET_ITEM_2,
                TOLERANCE,
            },
        },
        triggers::{
            CompareOp, CompareOperand, Op, RoundMode, SignMode, counter_object, item_edit,
            toggle_trigger,
        },
    },
};
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
                    format!("test"),
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
                    format!("test"),
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
                    format!("test"),
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
                    format!("test"),
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
                    format!("test"),
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
                    format!("test"),
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

// TODO: get rid for this when the gdlib maintainer fixes his crate
pub fn item_compare(
    config: &GDObjConfig,
    true_id: i16,
    false_id: i16,
    lhs: CompareOperand,
    rhs: CompareOperand,
    compare_op: CompareOp,
    tolerance: f64,
) -> GDObject {
    let properties = vec![
        (TARGET_ITEM, GDValue::Item(true_id)),
        (TARGET_ITEM_2, GDValue::Item(false_id)),
        // ids
        (INPUT_ITEM_1, GDValue::Item(lhs.operand_item.id())),
        (INPUT_ITEM_2, GDValue::Item(rhs.operand_item.id())),
        // types
        (
            FIRST_ITEM_TYPE,
            GDValue::Int(lhs.operand_item.get_type_as_i32()),
        ),
        (
            SECOND_ITEM_TYPE,
            GDValue::Int(rhs.operand_item.get_type_as_i32()),
        ),
        // modifiers
        (MODIFIER, GDValue::Float(lhs.modifier)),
        (SECOND_MODIFIER, GDValue::Float(rhs.modifier)),
        // modifiers ops
        (LEFT_OPERATOR, GDValue::Int(lhs.mod_op as i32)),
        (RIGHT_OPERATOR, GDValue::Int(rhs.mod_op as i32)),
        (COMPARE_OPERATOR, GDValue::Int(compare_op as i32)),
        (TOLERANCE, GDValue::Float(tolerance)),
        // round modes
        (LEFT_ROUND_MODE, GDValue::Int(lhs.rounding as i32)),
        (RIGHT_ROUND_MODE, GDValue::Int(rhs.rounding as i32)),
        // sign modes
        (LEFT_SIGN_MODE, GDValue::Int(lhs.sign as i32)),
        (RIGHT_SIGN_MODE, GDValue::Int(rhs.sign as i32)),
    ];

    GDObject::new(TRIGGER_ITEM_COMPARE, config, properties)
}

#[test]
fn generate_memblock() {
    // generates the block of memory of vmem (value-based memory)
    let memsize = 100;
    let ptrpos = 99;
    let memreg = 98;

    let memblock_height = f64::sqrt(memsize as f64).ceil() as i16;
    let max_bits = f64::log2(memsize as f64).ceil() as i16;
    // group layout:
    // * 1: read group
    // * 2: write group
    // * 3..(3 + maxbits * 2): encoding for bits of counter.
    // if group is odd, then bit is 0
    // if group is even, then bit is 1
    // e.g. group 3: first bit is 0, group 4: first bit is 1
    // bits grouped like this (ordering by least significant bit of ID):
    // * first: [3 4]
    // * second: [5 6]
    // * third: [7 8]
    // * fourth: [9 10]
    // * and so on
    // one last group to spawn the controller (see below)
    // groups used in total: [1, 4 + maxbits * 2]

    let read_group = 1;
    let write_group = 2;

    let mut level = Level::new("vmem", "tasmc", None, None);
    for i in 0..memsize {
        // groups are binary encoded
        let mut groups: Vec<i16> = vec![0; max_bits as usize];

        let mut group = 3;
        for g_idx in 0..max_bits {
            // the logic here:
            // add odd group (group + 0, since it is already odd) if bit == 0
            // add even group (group + 1, to make it even) if bit == 1
            // since we are adding the value of the bit to the group,
            // we can avoid the if statement.
            let bit = (i >> g_idx) & 1;
            groups[g_idx as usize] = group + bit;
            group += 2;
        }

        let cfg = GDObjConfig::new()
            .pos(
                ((i) % memblock_height + 2) as f64 * 15.0 + 15.0,
                ((i) / memblock_height + 2) as f64 * 15.0 + 15.0,
            )
            .scale(0.2, 0.2)
            .groups(groups)
            .spawnable(true)
            .multitrigger(true);

        level.add_object(counter_object(
            &cfg.clone().spawnable(false).multitrigger(false),
            Item::Counter(i as i16 + 1),
            gdlib::gdobj::triggers::ItemAlign::Center,
            false,
        ));
        // read: item -> memreg
        let mut read_cfg = cfg.clone();
        read_cfg.add_group(Group::Regular(read_group));

        level.add_object(item_edit(
            &read_cfg,
            Some(Item::Counter(i as i16 + 1)),
            None,
            Item::Counter(memreg),
            1.0,
            Op::Set,
            true,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));
        // write: item <- memreg
        let mut write_cfg = cfg.clone();
        write_cfg.add_group(Group::Regular(write_group));
        level.add_object(item_edit(
            &write_cfg,
            Some(Item::Counter(memreg)),
            None,
            Item::Counter(i as i16 + 1),
            1.0,
            Op::Set,
            true,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));
    }

    /* the controller:
     * this thing is what toggles off everything but what we need.
     * it essentially does a bitmask for each bit to see what to toggle off
     * e.g. 0b10 (2)
     *  - this toggles off the opposite bit's group for each bit
     *  - toggles off the groups that correspond to 1 for the least significant bit
     *    and 0 for the second least significant bit
     *
     * before running it, we must toggle everything else back on
     * from the previous time it was ran
     */

    // starting height for the controller area
    let starting_height = (memblock_height + 4) as f64 * 15.0 + 45.0;
    let mut next_group = 3 + max_bits * 2;
    let controller_group = next_group;
    next_group += 1;

    // controller group is the next group available

    // toggle all groups on
    // having a master group to toggle everything on at once doesn't work
    // so we toggle everything back on individually
    for group in 1..(3 + max_bits * 2) {
        level.add_object(toggle_trigger(
            &GDObjConfig::new()
                .pos(75.0, starting_height - 30.0)
                .groups([controller_group])
                .spawnable(true)
                .multitrigger(true),
            group,
            true,
        ));
    }

    // mask each bit
    for bit in 0..max_bits {
        let mut cfg = GDObjConfig::new()
            .pos(75.0, starting_height + (bit as f64) * 30.0)
            .groups([controller_group])
            .spawnable(true)
            .multitrigger(true);

        // isolate bit
        let temp_ctr = Item::Counter(memreg - (bit + 1));
        level.add_object(item_edit(
            &cfg,
            Some(Item::Counter(ptrpos)),
            None,
            temp_ctr.clone(),
            (bit as f64).exp2(),
            Op::Set,
            false,
            None,
            RoundMode::Floor,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));

        level.add_object(item_compare(
            &cfg.clone().translate(1.0, 0.0),
            next_group,
            next_group + 1,
            CompareOperand {
                operand_item: temp_ctr.clone(),
                modifier: 2.0,
                mod_op: Op::Div,
                rounding: RoundMode::Floor,
                sign: SignMode::None,
            },
            CompareOperand {
                operand_item: temp_ctr.clone(),
                modifier: 2.0,
                mod_op: Op::Div,
                rounding: RoundMode::None,
                sign: SignMode::None,
            },
            CompareOp::Equals,
            0.0,
        ));

        cfg = cfg.translate(30.0, 0.0).groups([next_group]);
        level.add_object(toggle_trigger(&cfg, bit * 2 + 4, false));
        cfg = cfg.translate(30.0, 0.0).groups([next_group + 1]);
        level.add_object(toggle_trigger(&cfg, bit * 2 + 3, false));
        next_group += 2;
    }

    level.export_to_gmd("vmem.gmd").unwrap();
}
