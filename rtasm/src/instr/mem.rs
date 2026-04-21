use std::iter;

use gdlib::gdobj::{
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
    misc::{default_block, text},
    triggers::{
        ColliderConfig, CompareOp, CompareOperand, DefaultMove, ItemAlign, MoveMode, MoveTarget,
        Op, RoundMode, SignMode, TargetMove, collision_block, collision_trigger, counter_object,
        item_edit, move_trigger, spawn_trigger, toggle_trigger,
    },
};

use crate::{
    core::{
        HandlerReturn,
        error::{TasmError, TasmErrorType},
        structs::{HandlerArgs, HandlerData, MemInfo, MemType, TasmValue},
    },
    instr::GROUP_SPAWN_DELAY,
};

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

pub fn legacy_malloc_inner(args: HandlerArgs, float_mem: bool) -> HandlerData {
    let (mem_x, mem_y) = (45.0, 165.0 + args.routine_count as f64 * 30.0);
    let mem_size = args.args[0].to_int().unwrap() as i16;

    let start_counter_id = args.ptrpos_id - mem_size - 1;
    let ptr_collblock_id = mem_size + 1;
    let memreg_id = args.ptrpos_id - 1;

    let mut next_free_group = args.curr_group;

    let ptr_reset_group = next_free_group;
    let ptr_group = next_free_group + 1;
    next_free_group += 2;

    let read_group = next_free_group;
    let write_group = next_free_group + 1;
    next_free_group += 2;

    let block_cfg = &GDObjConfig::new()
        .pos(mem_x, mem_y - 30.0)
        .scale(0.5, 0.5)
        .groups([ptr_reset_group]);

    let mut objs = vec![
        // reset block
        default_block(block_cfg),
        // pointer block
        collision_block(
            &block_cfg.clone().groups([ptr_group]).scale(0.8, 0.8),
            ptr_collblock_id,
            true,
        ),
    ];

    let mut idx = 0i16;
    let mut counter_id = start_counter_id;

    let memreg_item = match float_mem {
        true => Item::Timer(memreg_id),
        false => Item::Counter(memreg_id),
    };
    let ptrpos_item = Item::Counter(args.ptrpos_id);

    while counter_id < memreg_id {
        let item_group = next_free_group;
        let collblock_id = idx + 1;
        let xpos = idx as f64 * 30.0 + mem_x;

        let counter_item = match float_mem {
            true => Item::Timer(counter_id),
            false => Item::Counter(counter_id),
        };

        let mut cfg = GDObjConfig::new().pos(xpos, mem_y);

        objs.push(collision_block(&cfg, collblock_id, false));
        cfg = cfg
            .pos(mem_x - 71.25, mem_y + (idx + 1) as f64 * 7.5 - 18.75)
            .groups([item_group])
            .scale(0.25, 0.25);
        objs.push(collision_trigger(
            &cfg,
            ColliderConfig::two_colliders(collblock_id, ptr_collblock_id),
            item_group,
            true,
            false,
        ));
        cfg = cfg
            .pos(xpos, mem_y + 30.0)
            .groups([item_group, write_group])
            .spawnable(true)
            .multitrigger(true)
            .scale(1.0, 1.0);
        // write memreg to item
        objs.push(item_edit(
            &cfg,
            Some(memreg_item),
            None,
            counter_item,
            1.0,
            Op::Set,
            false,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));
        // read item to memreg
        cfg = cfg.y(mem_y + 60.0).groups([item_group, read_group]);
        // write memreg to item
        objs.push(item_edit(
            &cfg,
            Some(counter_item),
            None,
            memreg_item,
            1.0,
            Op::Set,
            false,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));
        // moves ptr back after is moves up
        cfg = cfg.y(mem_y + 90.0).groups([item_group]);
        objs.push(move_trigger(
            &cfg,
            MoveMode::Default(DefaultMove {
                dx: 0.0,
                dy: -30.0,
                x_lock: None,
                y_lock: None,
            }),
            0.0,
            ptr_group,
            false,
            false,
            None,
        ));

        // counter obj
        cfg = cfg
            .y(mem_y - 60.0)
            .groups(iter::empty::<i16>())
            .scale(0.4, 0.4)
            .angle(-30.0);
        objs.push(counter_object(&cfg, counter_item, ItemAlign::Center, false));

        next_free_group += 1;
        counter_id += 1;
        idx += 1;
    }

    objs.extend_from_slice(&[
        // memreg and ptrpos counters
        counter_object(
            &GDObjConfig::new()
                .pos(mem_x + mem_size as f64 * 30.0, mem_y - 60.0)
                .scale(0.4, 0.4)
                .angle(-30.0),
            memreg_item,
            ItemAlign::Center,
            false,
        ),
        counter_object(
            &GDObjConfig::new()
                .pos(mem_x + (mem_size + 1) as f64 * 30.0, mem_y - 60.0)
                .scale(0.4, 0.4)
                .angle(-30.0),
            ptrpos_item,
            ItemAlign::Center,
            false,
        ),
        // memory text
        text(
            &GDObjConfig::new().pos(mem_x, mem_y + 150.0).scale(0.5, 0.5),
            "memory",
            0,
        ),
    ]);

    // 1. each memory cell gets a column
    // 2. mem ptr and mem ptr reset <- dont forget to include these in return
    // 3. memory text
    // 4. memreg and ptrpos counters
    // 5. return memtype, used groups (next free - args.current), ptr reset group, meminfo

    let mut data = HandlerData::from_objects(objs);
    data.used_extra_groups = next_free_group - args.curr_group;
    data.ptr_reset_group = ptr_reset_group;
    data.ptr_group = ptr_group;
    data.new_mem = Some(MemInfo {
        _type: match float_mem {
            true => MemType::LegacyFloat,
            false => MemType::LegacyInt,
        },
        memreg: match float_mem {
            true => TasmValue::Timer(memreg_id),
            false => TasmValue::Counter(memreg_id),
        },
        size: mem_size,
        ptrpos: TasmValue::Counter(args.ptrpos_id),
        read_group,
        write_group,
        start_counter_id,
        line: args.line,
    });

    data
}

pub fn legacy_malloc(args: HandlerArgs) -> HandlerReturn {
    Ok(legacy_malloc_inner(args, false))
}
pub fn legacy_fmalloc(args: HandlerArgs) -> HandlerReturn {
    Ok(legacy_malloc_inner(args, true))
}

pub fn init_mem(args: HandlerArgs) -> HandlerReturn {
    let y_offset = args.routine_count as f64 * 30.0 + 150.0;
    let mut cfg = GDObjConfig::new().pos(-15.0, 0.0).scale(0.25, 0.25);

    let mem_info = args.mem_info.unwrap();
    let start_counter = mem_info.start_counter_id;

    let mut objs = vec![];

    for (idx, v) in args.args.iter().enumerate() {
        cfg = cfg.y(y_offset + 7.5 * (idx + 1) as f64 - 18.75);

        objs.push(item_edit(
            &cfg,
            None,
            None,
            match mem_info._type {
                MemType::Float | MemType::LegacyFloat => Item::Timer(start_counter + idx as i16),
                MemType::Int | MemType::LegacyInt => Item::Counter(start_counter + idx as i16),
            },
            v.to_float().unwrap(),
            Op::Set,
            false,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));
    }

    Ok(HandlerData::from_objects(objs))
}

pub fn legacy_mptr(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let move_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 + 7.5);
    let add_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 - 7.5);
    let move_amount = args.args[0].to_float().unwrap();
    let invalid_move_reason;
    let is_valid_mem_move = match args.mem_info {
        Some(mem) => {
            if move_amount as i16 <= mem.size {
                invalid_move_reason = String::new();
                true
            } else {
                invalid_move_reason = "Pointer moved more spaces than memory size".into();
                false
            }
        }
        None => {
            invalid_move_reason = "Pointer moved while no memory exists".into();
            false
        }
    };

    if is_valid_mem_move {
        Ok(HandlerData::from_objects(vec![
            move_trigger(
                &move_cfg,
                MoveMode::Default(DefaultMove {
                    dx: 30.0 * move_amount,
                    dy: 0.0,
                    x_lock: None,
                    y_lock: None,
                }),
                0.0,
                args.ptr_group,
                false,
                false,
                None,
            ),
            item_edit(
                &add_cfg,
                None,
                None,
                Item::Counter(args.ptrpos_id),
                move_amount,
                Op::Add,
                false,
                None,
                RoundMode::None,
                RoundMode::None,
                SignMode::None,
                SignMode::None,
            ),
        ]))
    } else {
        Err(TasmError {
            _type: TasmErrorType::InvalidPointerMove,
            file: String::new(),
            routine: String::new(),
            error: true,
            line: args.line,
            details: invalid_move_reason,
        })
    }
}

pub fn legacy_mreset(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let move_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 + 7.5);
    let add_cfg = cfg.clone().scale(1.0, 0.5).y(cfg.pos.1 - 7.5);
    Ok(HandlerData::from_objects(vec![
        move_trigger(
            &move_cfg,
            MoveMode::Targeting(TargetMove {
                target_group_id: MoveTarget::Group(args.ptr_reset_group),
                center_group_id: None,
                axis_only: None,
            }),
            0.0,
            args.ptr_group,
            false,
            false,
            None,
        ),
        item_edit(
            &add_cfg,
            None,
            None,
            Item::Counter(args.ptrpos_id),
            0.0,
            Op::Set,
            false,
            None,
            RoundMode::None,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ),
    ]))
}

pub fn legacy_mfunc(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![move_trigger(
        &args.cfg,
        MoveMode::Default(DefaultMove {
            dx: 0.0,
            dy: 30.0,
            x_lock: None,
            y_lock: None,
        }),
        0.0,
        args.ptr_group,
        false,
        false,
        None,
    )])
    .skip_spaces(2))
}

pub fn legacy_mem_mode(args: HandlerArgs, toggle_read: bool) -> HandlerReturn {
    let top_cfg = args.cfg.clone().scale(0.5, 0.5).y(args.cfg.pos.1 + 7.5);
    let bottom_cfg = args.cfg.clone().scale(0.5, 0.5).y(args.cfg.pos.1 - 7.5);
    let mem_info = args.mem_info.unwrap();

    Ok(HandlerData::from_objects(vec![
        toggle_trigger(&top_cfg, mem_info.write_group, !toggle_read),
        toggle_trigger(&bottom_cfg, mem_info.read_group, toggle_read),
    ]))
}

pub fn legacy_mwrite(args: HandlerArgs) -> HandlerReturn {
    legacy_mem_mode(args, false)
}
pub fn legacy_mread(args: HandlerArgs) -> HandlerReturn {
    legacy_mem_mode(args, true)
}

pub fn malloc_generator(args: HandlerArgs, float_mem: bool) -> HandlerData {
    // generates the block of memory of vmem (value-based memory)

    // memreg is 2nd last
    // ptrpos is last
    let start_ctr = args.args[0].to_int().unwrap() as i16;
    let end_ctr = args.args[0].to_int().unwrap() as i16;
    let memsize = start_ctr - end_ctr;

    let ptrpos = end_ctr;
    let memreg = end_ctr - 1;

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

    let item_constructor = match float_mem {
        true => Item::Timer,
        false => Item::Counter,
    };

    let group_offset = args.curr_group;
    let read_group = 1 + group_offset;
    let write_group = 2 + group_offset;

    let mut level = vec![];
    for i in 0..memsize {
        // groups are binary encoded
        let mut groups: Vec<i16> = vec![0; max_bits as usize];

        let mut group: i16 = 3 + group_offset;
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

        level.push(counter_object(
            &cfg.clone().spawnable(false).multitrigger(false),
            (item_constructor)(i + 1),
            gdlib::gdobj::triggers::ItemAlign::Center,
            false,
        ));
        // read: item -> memreg
        let mut read_cfg = cfg.clone();
        read_cfg.add_group(Group::Regular(read_group));

        level.push(item_edit(
            &read_cfg,
            Some((item_constructor)(i + 1)),
            None,
            (item_constructor)(memreg),
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
        level.push(item_edit(
            &write_cfg,
            Some((item_constructor)(memreg)),
            None,
            (item_constructor)(i + 1),
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
    let mut next_group = 3 + max_bits * 2 + group_offset;
    let controller_group = next_group;
    next_group += 1;

    // controller group is the next group available

    // toggle all groups on
    // having a master group to toggle everything on at once doesn't work
    // so we toggle everything back on individually
    for group in 1..(3 + max_bits * 2) {
        level.push(toggle_trigger(
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
        let temp_ctr = (item_constructor)(memreg - (bit + 1));
        level.push(item_edit(
            &cfg,
            Some(Item::Counter(ptrpos)),
            None,
            temp_ctr,
            (bit as f64).exp2(),
            Op::Set,
            false,
            None,
            RoundMode::Floor,
            RoundMode::None,
            SignMode::None,
            SignMode::None,
        ));

        level.push(item_compare(
            &cfg.clone().translate(1.0, 0.0),
            next_group,
            next_group + 1,
            CompareOperand {
                operand_item: temp_ctr,
                modifier: 2.0,
                mod_op: Op::Div,
                rounding: RoundMode::Floor,
                sign: SignMode::None,
            },
            CompareOperand {
                operand_item: temp_ctr,
                modifier: 2.0,
                mod_op: Op::Div,
                rounding: RoundMode::None,
                sign: SignMode::None,
            },
            CompareOp::Equals,
            0.0,
        ));

        cfg = cfg.translate(30.0, 0.0).groups([next_group]);
        level.push(toggle_trigger(&cfg, bit * 2 + 4, false));
        cfg = cfg.translate(30.0, 0.0).groups([next_group + 1]);
        level.push(toggle_trigger(&cfg, bit * 2 + 3, false));
        next_group += 2;
    }

    // objects are in level

    let mut data = HandlerData::from_objects(level);

    data.used_extra_groups = next_group - args.curr_group;
    // field used in legacy memory, not used in vmem.
    data.ptr_reset_group = 0;
    // controller group set as ptr group for vmem instructions.
    data.ptr_group = controller_group;

    data.new_mem = Some(MemInfo {
        _type: match float_mem {
            true => MemType::Float,
            false => MemType::Int,
        },
        memreg: match float_mem {
            true => TasmValue::Timer(memreg),
            false => TasmValue::Counter(memreg),
        },
        size: memsize,
        ptrpos: TasmValue::Counter(args.ptrpos_id),
        read_group,
        write_group,
        start_counter_id: start_ctr,
        line: args.line,
    });

    data
}

pub fn malloc(args: HandlerArgs) -> HandlerReturn {
    Ok(malloc_generator(args, false))
}
pub fn fmalloc(args: HandlerArgs) -> HandlerReturn {
    Ok(malloc_generator(args, true))
}

pub fn mset(args: HandlerArgs) -> HandlerReturn {
    // writing, so toggle off read group
    let minfo = args.mem_info.unwrap();
    Ok(HandlerData::from_objects(vec![
        spawn_trigger(
            &args.cfg,
            args.ptr_group,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
            vec![],
        ),
        toggle_trigger(
            &args.cfg.clone().translate(1.0, 0.0),
            minfo.read_group,
            false,
        ),
    ])
    .skip_spaces(4))
}

pub fn mget(args: HandlerArgs) -> HandlerReturn {
    // writing, so toggle off read group
    let minfo = args.mem_info.unwrap();
    Ok(HandlerData::from_objects(vec![
        spawn_trigger(
            &args.cfg,
            args.ptr_group,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
            vec![],
        ),
        toggle_trigger(
            &args.cfg.clone().translate(1.0, 0.0),
            minfo.write_group,
            false,
        ),
    ])
    .skip_spaces(4))
}
