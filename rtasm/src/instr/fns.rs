use gdlib::gdobj::{
    GDObjConfig, GDObject, ItemType, ZLayer,
    misc::{default_block, text},
    triggers::{
        CompareOp, CompareOperand, ItemAlign, Op, RoundMode, SignMode, StopMode, TimeTriggerConfig,
        counter_object, item_compare, item_edit, persistent_item, random_trigger, spawn_trigger,
        stop_trigger, time_control, time_trigger,
    },
};

use paste::paste;

use crate::{
    core::{
        HandlerReturn,
        error::{TasmError, TasmErrorType},
        flags::FlagValue,
        structs::{HandlerArgs, HandlerData},
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
                        HandlerData::from_objects($inner_fn(args, (LowerCompOp::$var).to_op()))
                            .extra_groups($extra_groups),
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

pub fn wait(args: HandlerArgs) -> HandlerReturn {
    // skip specified amount of spaces
    let wait = args.args[0].to_int().unwrap();
    if wait >= 0 {
        Ok(HandlerData::default().skip_spaces(args.args[0].to_int().unwrap()))
    } else {
        Err(TasmError {
            _type: TasmErrorType::InvalidWaitAmount,
            file: String::new(),
            routine: String::new(),
            error: true,
            line: args.line,
            details: "Cannot wait a negative number of ticks.".to_string(),
        })
    }
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
            RoundMode::Nearest
        } else {
            RoundMode::None
        },
        SignMode::None,
    );
    flag_override(&mut finmode, "finmode", &args);

    vec![item_edit(
        &args.cfg,
        Some(operand),
        None,
        result,
        modifier,
        get_flag_value(&args, "iter", FlagValue::Op(op)).into(),
        !get_flag_value(&args, "divmod", FlagValue::Bool(false))
            .to_bool()
            .unwrap(),
        get_flag_value_opt(&args, "op").map(|f| f.to_op().unwrap()),
        resmode.0,
        finmode.0,
        resmode.1,
        finmode.1,
    )]
}
pub fn arithmetic_3items(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();
    let op2 = get_item_spec(&args.args[2]).unwrap();

    let mut modifier = 1.0;
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (RoundMode::None, SignMode::None);
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (
        if round_res {
            RoundMode::Nearest
        } else {
            RoundMode::None
        },
        SignMode::None,
    );
    flag_override(&mut finmode, "finmode", &args);

    vec![item_edit(
        &args.cfg,
        Some(op1),
        Some(op2),
        res,
        modifier,
        get_flag_value(&args, "iter", FlagValue::Op(Op::Set)).into(),
        !get_flag_value(&args, "divmod", FlagValue::Bool(false))
            .to_bool()
            .unwrap(),
        Some(get_flag_value(&args, "op", FlagValue::Op(op)).into()),
        resmode.0,
        finmode.0,
        resmode.1,
        finmode.1,
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
            RoundMode::Nearest
        } else {
            RoundMode::None
        },
        SignMode::None,
    );
    flag_override(&mut finmode, "finmode", &args);

    vec![item_edit(
        &args.cfg,
        None,
        None,
        res,
        modifier,
        get_flag_value(&args, "iter", FlagValue::Op(op)).into(),
        !get_flag_value(&args, "divmod", FlagValue::Bool(false))
            .to_bool()
            .unwrap(),
        get_flag_value_opt(&args, "op").map(|f| f.to_op().unwrap()),
        resmode.0,
        finmode.0,
        resmode.1,
        finmode.1,
    )]
}
pub fn arithmetic_2items_num(args: HandlerArgs, op: Op, round_res: bool) -> Vec<GDObject> {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();
    let mut modifier = args.args[2].to_float().unwrap();
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (RoundMode::None, SignMode::None);
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (
        if round_res {
            RoundMode::Nearest
        } else {
            RoundMode::None
        },
        SignMode::None,
    );
    flag_override(&mut finmode, "finmode", &args);
    vec![item_edit(
        &args.cfg,
        Some(op1),
        None,
        res,
        modifier,
        get_flag_value(&args, "iter", FlagValue::Op(Op::Set)).into(),
        // since we know this is only used for mul and div instructions, this is fine.
        !get_flag_value(&args, "divmod", FlagValue::Bool(op == Op::Mul))
            .to_bool()
            .unwrap(),
        Some(get_flag_value(&args, "op", FlagValue::Op(op)).into()),
        resmode.0,
        finmode.0,
        resmode.1,
        finmode.1,
    )]
}

handlers!((add, sub, mul, div, mov) => arithmetic_2items);
handlers!((add, sub, mul, div) => arithmetic_3items);
handlers!((add, sub, mul, div, mov) => arithmetic_item_num);
handlers!((mul, div) => arithmetic_2items_num);

pub fn arithmetic_with_mod_2items_num(args: HandlerArgs, op: Op, mul: bool) -> GDObject {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();

    let mut modifier = args.args[2].to_float().unwrap();
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (RoundMode::None, SignMode::None);
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (RoundMode::None, SignMode::None);
    flag_override(&mut finmode, "finmode", &args);

    item_edit(
        &args.cfg,
        Some(op1),
        None,
        res,
        modifier,
        get_flag_value(&args, "iter", FlagValue::Op(op)).into(),
        !get_flag_value(&args, "divmod", FlagValue::Bool(mul))
            .to_bool()
            .unwrap(),
        get_flag_value_opt(&args, "op").map(|f| f.into()),
        resmode.0,
        finmode.0,
        resmode.1,
        finmode.1,
    )
}
pub fn arithmetic_with_mod_3items_num(args: HandlerArgs, op: Op, mul: bool) -> GDObject {
    let res = get_item_spec(&args.args[0]).unwrap();
    let op1 = get_item_spec(&args.args[1]).unwrap();
    let op2 = get_item_spec(&args.args[2]).unwrap();

    let mut modifier = args.args[3].to_float().unwrap();
    flag_override(&mut modifier, "itemmod", &args);
    let mut resmode = (RoundMode::None, SignMode::None);
    flag_override(&mut resmode, "resmode", &args);
    let mut finmode = (RoundMode::None, SignMode::None);
    flag_override(&mut finmode, "finmode", &args);

    item_edit(
        &args.cfg,
        Some(op1),
        Some(op2),
        res,
        modifier,
        get_flag_value(&args, "iter", FlagValue::Op(op)).into(),
        !get_flag_value(&args, "divmod", FlagValue::Bool(mul))
            .to_bool()
            .unwrap(),
        // id op should be the same as assign op
        Some(get_flag_value(&args, "op", FlagValue::Op(op)).into()),
        RoundMode::None,
        RoundMode::None,
        SignMode::None,
        SignMode::None,
    )
}

pub fn add_mod_2items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![
        arithmetic_with_mod_2items_num(args, Op::Add, true),
    ]))
}
pub fn add_mod_3items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![
        arithmetic_with_mod_3items_num(args, Op::Add, true),
    ]))
}
pub fn sub_mod_2items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![
        arithmetic_with_mod_2items_num(args, Op::Sub, true),
    ]))
}
pub fn sub_mod_3items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![
        arithmetic_with_mod_3items_num(args, Op::Sub, true),
    ]))
}
pub fn add_div_2items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![
        arithmetic_with_mod_2items_num(args, Op::Add, false),
    ]))
}
pub fn add_div_3items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![
        arithmetic_with_mod_3items_num(args, Op::Add, false),
    ]))
}
pub fn sub_div_2items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![
        arithmetic_with_mod_2items_num(args, Op::Sub, false),
    ]))
}
pub fn sub_div_3items_num(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![
        arithmetic_with_mod_3items_num(args, Op::Sub, false),
    ]))
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
    spawn_trigger(
        spawn_cfg,
        group,
        GROUP_SPAWN_DELAY,
        0.0,
        false,
        true,
        false,
        vec![],
    )
}

pub fn spawn_item_num(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);

    let iargs = args.args;
    let lhs = get_item_spec(&iargs[1]).unwrap();

    let spawning_group = iargs[0].to_group_id().unwrap();
    let spawn_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([args.curr_group])
        .set_control_id(spawning_group); // use auxiliary group for spawn trigger
    // SX rtn, I1, 42
    // args: [Group(n), ]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group, // spawn auxiliary group (spawn trigger)
            0,
            lhs.into(),
            CompareOperand::number_literal(iargs[2].to_float().unwrap()),
            op,
            0.0,
        ),
        spawn_trg(&spawn_cfg, spawning_group),
    ]
}
pub fn spawn_item_item(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);

    let iargs = args.args;
    let lhs = get_item_spec(&iargs[1]).unwrap();
    let rhs = get_item_spec(&iargs[2]).unwrap();
    let spawning_group = iargs[0].to_group_id().unwrap();
    let spawn_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([args.curr_group])
        .set_control_id(spawning_group); // use auxiliary group for spawn trigger
    // SX rtn, I1, 42
    // args: [Group(n), ]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group, // spawn auxiliary group (spawn trigger)
            0,
            lhs.into(),
            rhs.into(),
            op,
            0.0,
        ),
        spawn_trg(&spawn_cfg, spawning_group),
    ]
}
pub fn fork_item_num(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    // below
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1).scale(0.33, 0.33);

    let iargs = args.args;
    let lhs = get_item_spec(&iargs[2]).unwrap();
    let num = iargs[3].to_float().unwrap();

    let spawning_true = iargs[0].to_group_id().unwrap();
    let spawning_false = iargs[1].to_group_id().unwrap();
    let spawn_true_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group])
        .set_control_id(spawning_true); // use auxiliary group for spawn trigger

    let spawn_false_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 - 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group + 1])
        .set_control_id(spawning_false); // use auxiliary group for spawn trigger
    // FX rtn, rtn2, I1, 42
    // args: [Group(n), Group(n), Item, Number]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group,     // spawn auxiliary group (true trigger)
            args.curr_group + 1, // spawn 2nd aux group (false trigger)
            lhs.into(),
            CompareOperand::number_literal(num),
            op,
            0.0,
        ),
        spawn_trg(&spawn_true_cfg, spawning_true),
        spawn_trg(&spawn_false_cfg, spawning_false),
    ]
}
pub fn fork_item_item(args: HandlerArgs, op: CompareOp) -> Vec<GDObject> {
    // below
    let cfg = args.cfg;
    let compare_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1).scale(0.33, 0.33);

    let iargs = args.args;
    let lhs = get_item_spec(&iargs[2]).unwrap();
    let rhs = get_item_spec(&iargs[3]).unwrap();

    let spawning_true = iargs[0].to_group_id().unwrap();
    let spawning_false = iargs[1].to_group_id().unwrap();
    let spawn_true_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group])
        .set_control_id(spawning_true); // use auxiliary group for spawn trigger

    let spawn_false_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 - 10.0)
        .scale(0.33, 0.33)
        .groups([args.curr_group + 1])
        .set_control_id(spawning_false); // use auxiliary group for spawn trigger
    // FX rtn, rtn2, I1, 42
    // args: [Group(n), Group(n), Item, Item]

    vec![
        item_compare(
            &compare_cfg,
            args.curr_group,     // spawn auxiliary group (true trigger)
            args.curr_group + 1, // spawn 2nd aux group (false trigger)
            lhs.into(),
            rhs.into(),
            op,
            0.0,
        ),
        spawn_trg(&spawn_true_cfg, spawning_true),
        spawn_trg(&spawn_false_cfg, spawning_false),
    ]
}

handlers!([eq, ne, le, leq, ge, geq] + 1 => spawn_item_num);
handlers!([eq, ne, le, leq, ge, geq] + 1 => spawn_item_item);
handlers!([eq, ne, le, leq, ge, geq] + 2 => fork_item_num);
handlers!([eq, ne, le, leq, ge, geq] + 2 => fork_item_item);

/* RANDOMS */

pub fn spawn_random(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let random_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);

    let iargs = args.args;
    let spawning_group = iargs[0].to_group_id().unwrap();
    let chance = iargs[1].to_float().unwrap();

    let aux_group = args.curr_group;
    let spawn_cfg = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([aux_group])
        .set_control_id(spawning_group); // use auxiliary group for spawn trigger

    Ok(HandlerData::from_objects(vec![
        random_trigger(&random_cfg, chance, aux_group, 0),
        spawn_trg(&spawn_cfg, spawning_group),
    ])
    .extra_groups(1))
}

pub fn fork_random(args: HandlerArgs) -> HandlerReturn {
    let cfg = args.cfg;
    let random_cfg = cfg.clone().pos(cfg.pos.0, cfg.pos.1 - 7.5).scale(0.5, 0.5);

    let iargs = args.args;
    let spawning_group1 = iargs[0].to_group_id().unwrap();
    let spawning_group2 = iargs[1].to_group_id().unwrap();
    let chance = iargs[2].to_float().unwrap();

    let aux_group1 = args.curr_group;
    let aux_group2 = args.curr_group + 1;
    let spawn_cfg1 = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 + 7.5)
        .scale(0.5, 0.5)
        .groups([aux_group1])
        .set_control_id(spawning_group1); // use auxiliary group for spawn trigger
    let spawn_cfg2 = cfg
        .clone()
        .pos(cfg.pos.0, cfg.pos.1 - 7.5)
        .scale(0.5, 0.5)
        .groups([aux_group2])
        .set_control_id(spawning_group2); // use auxiliary group for spawn trigger

    Ok(HandlerData::from_objects(vec![
        random_trigger(&random_cfg, chance, aux_group1, aux_group2),
        spawn_trg(&spawn_cfg1, spawning_group1),
        spawn_trg(&spawn_cfg2, spawning_group2),
    ])
    .extra_groups(2))
}

/* PROCESS */

pub fn spawn(args: HandlerArgs) -> HandlerReturn {
    let spawning_group = args.args[0].to_group_id().unwrap();
    let cfg = args.cfg.clone().set_control_id(spawning_group);
    wrap_objs!(vec![spawn_trigger(
        &cfg,
        spawning_group,
        get_flag_value(&args, "delay", FlagValue::Float(GROUP_SPAWN_DELAY)).into(),
        0.0,
        false,
        true,
        false,
        get_flag_value(&args, "remap", FlagValue::Dict(vec![])).into()
    )])
}

pub fn pause(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![stop_trigger(
        &args.cfg,
        args.args[0].to_group_id().unwrap(),
        StopMode::Pause,
        true,
    )]))
}

pub fn resume(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![stop_trigger(
        &args.cfg,
        args.args[0].to_group_id().unwrap(),
        StopMode::Resume,
        true,
    )]))
}

pub fn stop(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![stop_trigger(
        &args.cfg,
        args.args[0].to_group_id().unwrap(),
        StopMode::Stop,
        true,
    )]))
}

/* TIMERS */

pub fn tstart(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![time_control(
        &args.cfg,
        get_item_spec(&args.args[0]).unwrap().id(),
        false,
    )]))
}

pub fn tstop(args: HandlerArgs) -> HandlerReturn {
    Ok(HandlerData::from_objects(vec![time_control(
        &args.cfg,
        get_item_spec(&args.args[0]).unwrap().id(),
        true,
    )]))
}

pub fn tspawn(args: HandlerArgs) -> HandlerReturn {
    let timer = args.args[0].to_timer_id().unwrap();
    let start_time = args.args[1].to_float().unwrap();
    let stop_time = args.args[2].to_float().unwrap();
    Ok(HandlerData::from_objects(vec![time_trigger(
        &args.cfg,
        TimeTriggerConfig {
            start_time,
            stop_time,
            pause_when_reached: get_flag_value(&args, "tstop", FlagValue::Bool(false)).into(),
            time_mod: get_flag_value(&args, "tmod", FlagValue::Float(1.0)).into(),
            timer_id: timer,
            ignore_timewarp: false,
            start_paused: get_flag_value(&args, "tpaused", FlagValue::Bool(false)).into(),
            dont_override: get_flag_value(&args, "nover", FlagValue::Bool(false)).into(),
        },
        args.args[3].to_group_id().unwrap(),
    )]))
}

/* INITS */

pub fn display(args: HandlerArgs) -> HandlerReturn {
    let item = get_item_spec(&args.args[0]).unwrap();
    let cfg = GDObjConfig::new()
        .pos(-75.0, 75.0 + 30.0 * args.displayed_items as f64)
        .scale(0.5, 0.5);

    let obj = counter_object(&cfg, item, ItemAlign::Center, false);

    Ok(HandlerData::from_objects(vec![obj])
        .skip_spaces(0)
        .added_item_display())
}

pub fn ioblock(args: HandlerArgs) -> HandlerReturn {
    let spawn_group = args.args[0].to_group_id().unwrap();
    let position = args.args[1].to_int().unwrap();
    let msg = args.args[2].to_string().unwrap();
    let cfg = GDObjConfig::new().pos(75.0 + position as f64 * 30.0, 75.0);
    let text_cfg = cfg.clone().scale(0.25, 0.25).set_z_layer(ZLayer::T2);
    let spawn_cfg = cfg
        .clone()
        .touchable(true)
        .multitrigger(true)
        .set_control_id(spawn_group);

    Ok(HandlerData::from_objects(vec![
        default_block(&cfg),
        spawn_trigger(
            &spawn_cfg,
            spawn_group,
            GROUP_SPAWN_DELAY,
            0.0,
            false,
            true,
            false,
            vec![],
        ),
        text(&text_cfg, msg, 0),
    ])
    .skip_spaces(0))
}

pub fn pers(args: HandlerArgs) -> HandlerReturn {
    let item = get_item_spec(&args.args[0]).unwrap();
    Ok(HandlerData::from_objects(vec![persistent_item(
        &args.cfg,
        item.id(),
        item.get_type() == ItemType::Timer,
        true,
        false,
        false,
    )]))
}
