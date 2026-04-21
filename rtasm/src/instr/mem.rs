use std::iter;

use gdlib::gdobj::{
    GDObjConfig, Item,
    misc::{default_block, text},
    triggers::{
        ColliderConfig, DefaultMove, ItemAlign, MoveMode, MoveTarget, Op, RoundMode, SignMode,
        TargetMove, collision_block, collision_trigger, counter_object, item_edit, move_trigger,
        toggle_trigger,
    },
};

use crate::core::{
    HandlerReturn,
    error::{TasmError, TasmErrorType},
    structs::{HandlerArgs, HandlerData, MemInfo, MemType, TasmValue},
};

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
            true => MemType::Float,
            false => MemType::Int,
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
                MemType::Float => Item::Timer(start_counter + idx as i16),
                MemType::Int => Item::Counter(start_counter + idx as i16),
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
