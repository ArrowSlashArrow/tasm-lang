use gdlib::cclocallevels::gdobj::{
    CollisionTrigger, CounterLabel, GDObject, InstantCollTrigger, ItemCompareTrigger,
    ItemEditTrigger, PersistentItemTrigger, RandomTrigger, SpawnTrigger, StopTrigger,
    TimeControlTrigger, TimeTrigger, ToggleTrigger,
    ids::objects::BLACK_GRADIENT_SQUARE,
    meta::GDObjConfig,
    structs::{
        CompareOp, CompareOperand, Group, ItemAlign, ItemType, Op, RoundMode, SignMode, StopMode,
        ZLayer,
    },
    text,
};

use paste::paste;

use crate::{
    core::{
        HandlerReturn,
        error::{TasmError, TasmErrorType},
        flags::FlagValue,
        structs::{HandlerArgs, HandlerData, Hitbox, validate_hitboxes},
    },
    instr::{
        GROUP_SPAWN_DELAY, LowerCompOp, LowerOp, flag_override, get_flag_value, get_flag_value_opt,
        get_item_spec,
    },
};

macro_rules! handlers {
    // handlers!((add, sub, mul, div) => _arith_2items)
    // variant: (arithmetic), [compare]; the var is lowercase and is converted.
    // for each argument, make a new fn that calls the inner fn
    // and returns the proper result type
    ( ($($var:ident),* $(,)?) => $inner_fn:ident) => {
        $(
            paste! {
                pub fn [<$inner_fn _ $var>](args: HandlerArgs) -> HandlerReturn {
                    Ok(HandlerData::from_objects($inner_fn(args, (LowerOp::$var).to_op(), false)))
                }
            }
        )*
    };

    ( [$($var:ident),* $(,)?] + $extra_groups:literal => $inner_fn:ident) => {
        $(
            paste! {
                pub fn [<$inner_fn _ $var>](args: HandlerArgs) -> HandlerReturn {
                    Ok(
                        HandlerData::from_objects($inner_fn(args, (LowerCompOp::$var).to_op(), false))
                            .extra_groups($extra_groups),
                    )
                }
                pub fn [<instant _ $inner_fn _ $var>](args: HandlerArgs) -> HandlerReturn {
                    Ok(
                        HandlerData::from_objects($inner_fn(args, (LowerCompOp::$var).to_op(), true)),
                    )
                }
            }
        )*
    };
}

macro_rules! wrap_objs {
    ($objs:expr) => {
        Ok(HandlerData::from_objects($objs))
    };
}

// useful for instructions that don't correspond to any objects
// namely debug instructions
// namely breakpoint
pub fn skip(_args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::default().skip_spaces(0))
}

/* WAIT */

pub fn nop(_args: HandlerArgs) -> HandlerReturn {
    // skip no-op space
    Ok(HandlerData::default().skip_spaces(1))
}

fn wait_internal(ticks: i32, line: usize) -> HandlerReturn {
    // skip specified amount of spaces
    if ticks >= 0 {
        Ok(HandlerData::default().skip_spaces(ticks))
    } else {
        Err(TasmError {
            etype: TasmErrorType::InvalidWaitAmount,
            file: String::new(),
            routine: String::new(),
            line,
            details: "Cannot wait a negative number of ticks.".to_string(),
            errcode: 6,
        })
    }
}

pub fn wait(args: HandlerArgs) -> HandlerReturn {
    wait_internal(args.args[0].to_int().unwrap(), args.line)
}

pub fn waits(args: HandlerArgs) -> HandlerReturn {
    wait_internal((args.args[0].to_float().unwrap() * 240.0) as i32, args.line)
}

/* ARITHMETIC */
// even though all functions return one object, they return Vecs for compatibility with the macro.
pub fn arithmetic_2items(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let result = get_item_spec(&args.args[0]).unwrap();
    let operand = get_item_spec(&args.args[1]).unwrap();

    let mut modifier = 1.0;
    flag_override(&mut modifier, "itemmod", &args);

    let mut resmode = (RoundMode::None, SignMode::None);
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (
        if round_res {
            RoundMode::Floor
        } else {
            RoundMode::None
        },
        SignMode::None,
    );
    flag_override(&mut finmode, "finmode", &args);

    vec![GDObject::from_config(
        args.cfg.clone(),
        ItemEditTrigger {
            operand1: Some(operand),
            operand2: None,
            target: result,
            modifier,
            assign_op: get_flag_value(&args, "iter", FlagValue::Op(op)).into(),
            multiply_mod: !get_flag_value(&args, "divmod", FlagValue::Bool(op == Op::Div))
                .to_bool()
                .unwrap(),
            id_op: get_flag_value(&args, "op", FlagValue::Op(op)).into(),
            id_rounding: resmode.0,
            result_rounding: finmode.0,
            id_sign: resmode.1,
            result_sign: finmode.1,
        },
    )]
}
pub fn arithmetic_3items(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();
    let op2 = get_item_spec(&args.args[2]).unwrap();

    let mut modifier = 1.0;
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (
        if round_res {
            RoundMode::Floor
        } else {
            RoundMode::None
        },
        SignMode::None,
    );
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (RoundMode::None, SignMode::None);
    flag_override(&mut finmode, "finmode", &args);

    vec![GDObject::from_config(
        args.cfg.clone(),
        ItemEditTrigger {
            operand1: Some(op1),
            operand2: Some(op2),
            target: res,
            modifier,
            assign_op: get_flag_value(&args, "iter", FlagValue::Op(Op::Set)).into(),
            multiply_mod: !get_flag_value(&args, "divmod", FlagValue::Bool(op == Op::Div))
                .to_bool()
                .unwrap(),
            id_op: get_flag_value(&args, "op", FlagValue::Op(op)).into(),
            id_rounding: resmode.0,
            result_rounding: finmode.0,
            id_sign: resmode.1,
            result_sign: finmode.1,
        },
    )]
}
pub fn arithmetic_item_num(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    // second arg should always be a number
    let mut modifier = args.args[1].to_float().unwrap();
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (RoundMode::None, SignMode::None);
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (
        if round_res {
            RoundMode::Floor
        } else {
            RoundMode::None
        },
        SignMode::None,
    );
    flag_override(&mut finmode, "finmode", &args);

    vec![GDObject::from_config(
        args.cfg.clone(),
        ItemEditTrigger {
            operand1: None,
            operand2: None,
            target: res,
            modifier,
            assign_op: get_flag_value(&args, "iter", FlagValue::Op(op)).into(),
            multiply_mod: !get_flag_value(&args, "divmod", FlagValue::Bool(op == Op::Div))
                .to_bool()
                .unwrap(),
            id_op: get_flag_value_opt(&args, "op")
                .map(|f| f.to_op().unwrap())
                .unwrap_or(op),
            id_rounding: resmode.0,
            result_rounding: finmode.0,
            id_sign: resmode.1,
            result_sign: finmode.1,
        },
    )]
}
pub fn arithmetic_2items_num(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();
    let mut modifier = args.args[2].to_float().unwrap();
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (
        if round_res {
            RoundMode::Floor
        } else {
            RoundMode::None
        },
        SignMode::None,
    );
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (RoundMode::None, SignMode::None);
    flag_override(&mut finmode, "finmode", &args);
    vec![GDObject::from_config(
        args.cfg.clone(),
        ItemEditTrigger {
            operand1: Some(op1),
            operand2: None,
            target: res,
            modifier,
            assign_op: get_flag_value(&args, "iter", FlagValue::Op(Op::Set)).into(),
            // since we know this is only used for mul and div instructions, this is fine.
            multiply_mod: !get_flag_value(&args, "divmod", FlagValue::Bool(op == Op::Div))
                .to_bool()
                .unwrap(),
            id_op: get_flag_value(&args, "op", FlagValue::Op(op)).into(),
            id_rounding: resmode.0,
            result_rounding: finmode.0,
            id_sign: resmode.1,
            result_sign: finmode.1,
        },
    )]
}

handlers!((add, sub, mul, div, mov) => arithmetic_2items);
handlers!((add, sub, mul, div) => arithmetic_3items);
handlers!((add, sub, mul, div, mov) => arithmetic_item_num);
handlers!((mul, div) => arithmetic_2items_num);

pub fn arithmetic_with_mod_2items_num(args: HandlerArgs, op: Op, mul: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();

    let mut modifier = args.args[2].to_float().unwrap();
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (RoundMode::None, SignMode::None);
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (RoundMode::None, SignMode::None);
    flag_override(&mut finmode, "finmode", &args);

    vec![GDObject::from_config(
        args.cfg.clone(),
        ItemEditTrigger {
            operand1: Some(op1),
            operand2: None,
            target: res,
            modifier,
            assign_op: get_flag_value(&args, "iter", FlagValue::Op(op)).into(),
            multiply_mod: !get_flag_value(&args, "divmod", FlagValue::Bool(mul))
                .to_bool()
                .unwrap(),
            id_op: get_flag_value(&args, "op", FlagValue::Op(op)).into(),
            id_rounding: resmode.0,
            result_rounding: finmode.0,
            id_sign: resmode.1,
            result_sign: finmode.1,
        },
    )]
}
pub fn arithmetic_with_mod_3items_num(args: HandlerArgs, op: Op, mul: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();
    let op2 = get_item_spec(&args.args[2]).unwrap();

    let mut modifier = args.args[3].to_float().unwrap();
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (RoundMode::None, SignMode::None);
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (RoundMode::None, SignMode::None);
    flag_override(&mut finmode, "finmode", &args);

    vec![GDObject::from_config(
        args.cfg.clone(),
        ItemEditTrigger {
            operand1: Some(op1),
            operand2: Some(op2),
            target: res,
            modifier,
            assign_op: get_flag_value(&args, "iter", FlagValue::Op(op)).into(),
            multiply_mod: !get_flag_value(&args, "divmod", FlagValue::Bool(mul))
                .to_bool()
                .unwrap(),
            // id op should be the same as assign op
            id_op: get_flag_value(&args, "op", FlagValue::Op(op)).into(),
            id_rounding: RoundMode::None,
            result_rounding: RoundMode::None,
            id_sign: SignMode::None,
            result_sign: SignMode::None,
        },
    )]
}

pub fn add_mod_2items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_with_mod_2items_num(args, Op::Add, true))
}
pub fn add_mod_3items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_with_mod_3items_num(args, Op::Add, true))
}
pub fn sub_mod_2items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_with_mod_2items_num(args, Op::Sub, true))
}
pub fn sub_mod_3items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_with_mod_3items_num(args, Op::Sub, true))
}
pub fn add_div_2items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_with_mod_2items_num(args, Op::Add, false))
}
pub fn add_div_3items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_with_mod_3items_num(args, Op::Add, false))
}
pub fn sub_div_2items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_with_mod_2items_num(args, Op::Sub, false))
}
pub fn sub_div_3items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_with_mod_3items_num(args, Op::Sub, false))
}

// fldiv instructions are not supported in the macro, so they are defined here.
pub fn fldiv_2items(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_2items(args, Op::Div, true,))
}
pub fn fldiv_item_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_item_num(args, Op::Div, true,))
}
pub fn fldiv_3items(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_3items(args, Op::Div, true,))
}
pub fn fldiv_2items_num(args: HandlerArgs) -> HandlerReturn {
    wrap_objs!(arithmetic_2items_num(args, Op::Div, true,))
}

/* COMPARES */

pub fn spawn_trg(spawn_cfg: &GDObjConfig, group: i16) -> GDObject {
    GDObject::from_config(
        spawn_cfg.clone(),
        SpawnTrigger {
            spawn_id: group,
            delay: GROUP_SPAWN_DELAY,
            delay_variation: 0.0,
            reset_remap: false,
            spawn_ordered: true,
            preview_disable: false,
            spawn_remaps: vec![],
        },
    )
}

pub fn spawn_compare(
    args: HandlerArgs,
    op: CompareOp,
    instant: bool,
    num_2nd_arg: bool,
) -> Vec<GDObject> {
    let cfg = args.cfg;
    let scale = match instant {
        false => 0.5,
        true => 1.0,
    };
    let compare_cfg = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 - 7.5)
        .with_scale(scale, scale);

    let iargs = args.args.as_ref();
    let lhs = get_item_spec(&iargs[1]).unwrap();
    let rhs = if num_2nd_arg {
        CompareOperand::number_literal(iargs[2].to_float().unwrap())
    } else {
        get_item_spec(&iargs[2]).unwrap().into()
    };

    let target_group = iargs[0].to_group_id().unwrap();
    let spawning_group = if instant {
        target_group
    } else {
        args.curr_group
    };
    let spawn_cfg = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .with_scale(0.5, 0.5)
        .with_groups([args.curr_group])
        .with_control_id(target_group); // use auxiliary group for spawn trigger

    let compare = GDObject::from_config(
        compare_cfg,
        ItemCompareTrigger {
            true_id: spawning_group, // spawn auxiliary group (spawn trigger)
            false_id: 0,
            lhs: lhs.into(),
            rhs,
            compare_op: op,
            tolerance: 0.0,
        },
    );

    if instant {
        // don't use any intermediate triggers if spawning instantly
        vec![compare]
    } else {
        vec![compare, spawn_trg(&spawn_cfg, spawning_group)]
    }
}

pub fn spawn_item_item(args: HandlerArgs, op: CompareOp, instant: bool) -> Vec<GDObject> {
    spawn_compare(args, op, instant, false)
}
pub fn spawn_item_num(args: HandlerArgs, op: CompareOp, instant: bool) -> Vec<GDObject> {
    spawn_compare(args, op, instant, true)
}

pub fn fork_compare(
    args: HandlerArgs,
    op: CompareOp,
    instant: bool,
    num_2nd_arg: bool,
) -> Vec<GDObject> {
    // args for a fork compare: true, false, lhs, rhs
    let cfg = args.cfg;
    let scale = match instant {
        false => 0.33,
        true => 1.0,
    };
    let compare_cfg = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1)
        .with_scale(scale, scale);

    let iargs = args.args.as_ref();
    let lhs = get_item_spec(&iargs[2]).unwrap();
    let rhs = if num_2nd_arg {
        CompareOperand::number_literal(iargs[3].to_float().unwrap())
    } else {
        get_item_spec(&iargs[3]).unwrap().into()
    };

    let target_true = iargs[0].to_group_id().unwrap();
    let target_false = iargs[1].to_group_id().unwrap();

    let spawning_true = if instant {
        target_true
    } else {
        args.curr_group
    };
    let spawning_false = if instant {
        target_false
    } else {
        args.curr_group + 1
    };

    let spawn_true_cfg = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 + 10.0)
        .with_scale(0.33, 0.33)
        .with_groups([args.curr_group])
        .with_control_id(target_true); // use auxiliary group for spawn trigger

    let spawn_false_cfg = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 - 10.0)
        .with_scale(0.33, 0.33)
        .with_groups([args.curr_group + 1])
        .with_control_id(target_false); // use auxiliary group for spawn trigger

    let compare = GDObject::from_config(
        compare_cfg,
        ItemCompareTrigger {
            true_id: spawning_true,
            false_id: spawning_false,
            lhs: lhs.into(),
            rhs,
            compare_op: op,
            tolerance: 0.0,
        },
    );

    if instant {
        // don't use any intermediate triggers if spawning instantly
        vec![compare]
    } else {
        vec![
            compare,
            spawn_trg(&spawn_true_cfg, spawning_true),
            spawn_trg(&spawn_false_cfg, spawning_false),
        ]
    }
}

pub fn fork_item_num(args: HandlerArgs, op: CompareOp, instant: bool) -> Vec<GDObject> {
    fork_compare(args, op, instant, true)
}
pub fn fork_item_item(args: HandlerArgs, op: CompareOp, instant: bool) -> Vec<GDObject> {
    fork_compare(args, op, instant, false)
}

handlers!([eq, ne, le, leq, ge, geq] + 1 => spawn_item_num);
handlers!([eq, ne, le, leq, ge, geq] + 1 => spawn_item_item);
handlers!([eq, ne, le, leq, ge, geq] + 2 => fork_item_num);
handlers!([eq, ne, le, leq, ge, geq] + 2 => fork_item_item);

/* RANDOMS */

pub fn spawn_random(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let random_cfg = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 - 7.5)
        .with_scale(0.5, 0.5);

    let iargs = args.args.as_ref();
    let spawning_group = iargs[0].to_group_id().unwrap();
    let chance = iargs[1].to_float().unwrap();

    let aux_group = args.curr_group;
    let spawn_cfg = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .with_scale(0.5, 0.5)
        .with_groups([aux_group])
        .with_control_id(spawning_group); // use auxiliary group for spawn trigger

    Ok(HandlerData::from_objects(vec![
        GDObject::from_config(
            random_cfg,
            RandomTrigger {
                chance,
                target_group1: aux_group,
                target_group2: 0,
            },
        ),
        spawn_trg(&spawn_cfg, spawning_group),
    ])
    .extra_groups(1))
}

pub fn fork_random(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let random_cfg = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 - 7.5)
        .with_scale(0.5, 0.5);

    let iargs = args.args.as_ref();
    let spawning_group1 = iargs[0].to_group_id().unwrap();
    let spawning_group2 = iargs[1].to_group_id().unwrap();
    let chance = iargs[2].to_float().unwrap();

    let aux_group1 = args.curr_group;
    let aux_group2 = args.curr_group + 1;
    let spawn_cfg1 = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .with_scale(0.5, 0.5)
        .with_groups([aux_group1])
        .with_control_id(spawning_group1); // use auxiliary group for spawn trigger
    let spawn_cfg2 = cfg
        .clone()
        .with_pos(cfg.pos.0, cfg.pos.1 - 7.5)
        .with_scale(0.5, 0.5)
        .with_groups([aux_group2])
        .with_control_id(spawning_group2); // use auxiliary group for spawn trigger

    Ok(HandlerData::from_objects(vec![
        GDObject::from_config(
            random_cfg,
            RandomTrigger {
                chance,
                target_group1: aux_group1,
                target_group2: aux_group2,
            },
        ),
        spawn_trg(&spawn_cfg1, spawning_group1),
        spawn_trg(&spawn_cfg2, spawning_group2),
    ])
    .extra_groups(2))
}

pub fn instant_spawn_random(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let random_cfg = cfg.clone().with_pos(cfg.pos.0, cfg.pos.1 - 7.5);

    let iargs = args.args.as_ref();
    let spawning_group = iargs[0].to_group_id().unwrap();
    let chance = iargs[1].to_float().unwrap();

    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        random_cfg,
        RandomTrigger {
            chance,
            target_group1: spawning_group,
            target_group2: 0,
        },
    )]))
}

pub fn instant_fork_random(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let random_cfg = cfg.clone().with_pos(cfg.pos.0, cfg.pos.1 - 7.5);

    let iargs = args.args.as_ref();
    let spawning_group1 = iargs[0].to_group_id().unwrap();
    let spawning_group2 = iargs[1].to_group_id().unwrap();
    let chance = iargs[2].to_float().unwrap();

    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        random_cfg,
        RandomTrigger {
            chance,
            target_group1: spawning_group1,
            target_group2: spawning_group2,
        },
    )]))
}

/* PROCESS */

pub fn spawn(args: HandlerArgs) -> HandlerReturn {
    let spawning_group = args.args[0].to_group_id().unwrap();
    let cfg = args.cfg.clone().with_control_id(spawning_group);

    wrap_objs!(vec![GDObject::from_config(
        cfg,
        SpawnTrigger {
            spawn_id: spawning_group,
            delay: get_flag_value(&args, "delay", FlagValue::Float(GROUP_SPAWN_DELAY)).into(),
            delay_variation: 0.0,
            reset_remap: get_flag_value(&args, "noremap", FlagValue::Bool(false)).into(),
            spawn_ordered: get_flag_value(&args, "ordered", FlagValue::Bool(true)).into(),
            preview_disable: false,
            spawn_remaps: get_flag_value(&args, "remap", FlagValue::Dict(vec![])).into(),
        }
    )])
}

pub fn pause(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        StopTrigger {
            target_group: args.args[0].to_group_id().unwrap(),
            stop_mode: StopMode::Pause,
            use_control_id: true,
        },
    )]))
}

pub fn resume(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        StopTrigger {
            target_group: args.args[0].to_group_id().unwrap(),
            stop_mode: StopMode::Resume,
            use_control_id: true,
        },
    )]))
}

pub fn stop(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        StopTrigger {
            target_group: args.args[0].to_group_id().unwrap(),
            stop_mode: StopMode::Stop,
            use_control_id: true,
        },
    )]))
}

/* TIMERS */

pub fn tstart(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        TimeControlTrigger {
            id: get_item_spec(&args.args[0]).unwrap().id(),
            stop: false,
        },
    )]))
}

pub fn tstop(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        TimeControlTrigger {
            id: get_item_spec(&args.args[0]).unwrap().id(),
            stop: true,
        },
    )]))
}

pub fn tspawn(args: HandlerArgs) -> HandlerReturn {
    let timer = args.args[0].to_timer_id().unwrap();
    let start_time = args.args[1].to_float().unwrap();
    let stop_time = args.args[2].to_float().unwrap();
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg.clone(),
        TimeTrigger {
            start_time,
            stop_time,
            pause_when_reached: get_flag_value(&args, "tstop", FlagValue::Bool(false)).into(),
            time_mod: get_flag_value(&args, "tmod", FlagValue::Float(1.0)).into(),
            timer_id: timer,
            target_group: args.args[3].to_group_id().unwrap(),
            ignore_timewarp: false,
            start_paused: get_flag_value(&args, "tpaused", FlagValue::Bool(false)).into(),
            dont_override: get_flag_value(&args, "nover", FlagValue::Bool(false)).into(),
        },
    )]))
}

/* INITS */

pub fn display(args: HandlerArgs) -> HandlerReturn {
    let item = get_item_spec(&args.args[0]).unwrap();
    let cfg = GDObjConfig::new()
        .with_pos(-75.0, 75.0 + 30.0 * args.displayed_items as f64)
        .with_scale(0.5, 0.5);

    let obj = GDObject::from_config(
        cfg,
        CounterLabel {
            item,
            align: ItemAlign::Center,
            seconds_only: false,
        },
    );

    Ok(HandlerData::from_objects(vec![obj])
        .skip_spaces(0)
        .added_item_display())
}

pub fn ioblock(args: HandlerArgs) -> HandlerReturn {
    let spawn_group = args.args[0].to_group_id().unwrap();
    let position = args.args[1].to_int().unwrap();
    let msg = args.args[2].to_string().unwrap();
    let cfg = GDObjConfig::new().with_pos(75.0 + position as f64 * 30.0, 75.0);
    let text_cfg = cfg.clone().with_scale(0.25, 0.25).with_z_layer(ZLayer::T2);
    let spawn_cfg = cfg
        .clone()
        .with_touchable(true)
        .with_multitrigger(true)
        .with_control_id(spawn_group);

    Ok(HandlerData::from_objects(vec![
        GDObject::new(BLACK_GRADIENT_SQUARE, &cfg, vec![]),
        GDObject::from_config(
            spawn_cfg,
            SpawnTrigger {
                spawn_id: spawn_group,
                delay: GROUP_SPAWN_DELAY,
                delay_variation: 0.0,
                reset_remap: false,
                spawn_ordered: true,
                preview_disable: false,
                spawn_remaps: vec![],
            },
        ),
        text(&text_cfg, msg, 0),
    ])
    .skip_spaces(0))
}

pub fn pers(args: HandlerArgs) -> HandlerReturn {
    let item = get_item_spec(&args.args[0]).unwrap();
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        PersistentItemTrigger {
            item_id: item.id(),
            timer: item.get_type() == ItemType::Timer,
            persistent: true,
            target_all: false,
            reset: false,
        },
    )]))
}

pub fn unpers(args: HandlerArgs) -> HandlerReturn {
    let item = get_item_spec(&args.args[0]).unwrap();
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        PersistentItemTrigger {
            item_id: item.id(),
            timer: item.get_type() == ItemType::Timer,
            persistent: false,
            target_all: false,
            reset: false,
        },
    )]))
}

pub fn unpersall(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        PersistentItemTrigger {
            item_id: 0,
            timer: false,
            persistent: false,
            target_all: true,
            reset: false,
        },
    )]))
}

pub fn rpersall(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        PersistentItemTrigger {
            item_id: 0,
            timer: false,
            persistent: true,
            target_all: true,
            reset: true,
        },
    )]))
}

pub fn ton(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        ToggleTrigger {
            target_group: args.args[0].to_group_id().unwrap(),
            activate_group: true,
        },
    )]))
}

pub fn toff(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg,
        ToggleTrigger {
            target_group: args.args[0].to_group_id().unwrap(),
            activate_group: false,
        },
    )]))
}

pub fn raw_objs(args: HandlerArgs) -> HandlerReturn {
    // everything is irrelevant but the object string
    let objs = args.args[0]
        .to_string()
        .unwrap()
        .trim_matches(';')
        .split(';')
        .map(GDObject::parse_str)
        .collect::<Vec<GDObject>>();
    Ok(HandlerData::from_objects(objs).skip_spaces(0))
}

pub fn raw_trigger(args: HandlerArgs) -> HandlerReturn {
    let (x, y) = args.cfg.pos;
    let group = args.cfg.groups.get(0).unwrap_or(&Group::Regular(0));
    let objs = args.args[0]
        .to_string()
        .unwrap()
        .trim_matches(';')
        .split(';')
        .map(GDObject::parse_str)
        .map(|obj| {
            // inherit assigned position and group
            let mut o = obj;
            o.config = o
                .config
                .with_pos(x, y)
                .with_spawnable(true)
                .with_multitrigger(true);
            o.config.add_group(*group);
            o
        })
        .collect::<Vec<GDObject>>();

    Ok(HandlerData::from_objects(objs))
}

pub fn instcoll(args: HandlerArgs) -> HandlerReturn {
    let g1 = args.args[0].to_group_id().unwrap();
    let g2 = args.args[1].to_group_id().unwrap();
    let c1 = args.args[2].to_collblock().unwrap();
    let c2 = args.args[3].to_collblock().unwrap();

    if !validate_hitboxes(c1, c2) {
        return Err(TasmError {
            etype: TasmErrorType::InvalidInstruction,
            file: String::new(),
            routine: String::new(),
            line: args.line,
            details: format!("Cannot detect collision between {c1:?} and {c2:?}"),
            errcode: 48,
        });
    }

    let mut coll_cfg = InstantCollTrigger {
        true_id: g1,
        false_id: g2,
        ..Default::default()
    };

    if let Hitbox::Player2 = c2 {
        coll_cfg.collide_both_players = true;
    } else {
        coll_cfg.collider2 = c2.get_id().unwrap();
        match c1 {
            Hitbox::CollBlock(c) => coll_cfg.collider1 = c,
            Hitbox::Player1 => coll_cfg.collide_player1 = true,
            Hitbox::Player2 => coll_cfg.collide_player2 = true,
            Hitbox::EitherPlayer => {
                coll_cfg.collide_player1 = true;
                coll_cfg.collide_player2 = true;
            }
        }
    }

    return Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg, coll_cfg,
    )]));
}

pub fn coll(args: HandlerArgs) -> HandlerReturn {
    let g1 = args.args[0].to_group_id().unwrap();
    let c1 = args.args[1].to_collblock().unwrap();
    let c2 = args.args[2].to_collblock().unwrap();

    if !validate_hitboxes(c1, c2) {
        return Err(TasmError {
            etype: TasmErrorType::InvalidInstruction,
            file: String::new(),
            routine: String::new(),
            line: args.line,
            details: format!("Cannot detect collision between {c1:?} and {c2:?}"),
            errcode: 48,
        });
    }

    let mut coll_cfg = CollisionTrigger {
        target_id: g1,
        activate_group: !<FlagValue as Into<bool>>::into(get_flag_value(
            &args,
            "nospawn",
            FlagValue::Bool(false),
        )),
        on_trigger_exit: get_flag_value(&args, "onexit", FlagValue::Bool(false)).into(),
        ..Default::default()
    };

    if let Hitbox::Player2 = c2 {
        coll_cfg.collide_both_players = true;
    } else {
        coll_cfg.collider2 = c2.get_id().unwrap();
        match c1 {
            Hitbox::CollBlock(c) => coll_cfg.collider1 = c,
            Hitbox::Player1 => coll_cfg.collide_player1 = true,
            Hitbox::Player2 => coll_cfg.collide_player2 = true,
            Hitbox::EitherPlayer => {
                coll_cfg.collide_player1 = true;
                coll_cfg.collide_player2 = true;
            }
        }
    }

    return Ok(HandlerData::from_objects(vec![GDObject::from_config(
        args.cfg, coll_cfg,
    )]));
}
