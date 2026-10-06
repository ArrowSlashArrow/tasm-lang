use std::{collections::HashMap, fs::read_to_string};

use crate::Args;

use gdlib::{
    cclocallevels::{
        gdlevel::{GDLevel, leveldata::parse_objects},
        gdobj::{
            GDObject, ItemCompareTrigger, ItemEditTrigger, ObjectProperties, SpawnTrigger,
            ids::{objects::*, properties::*},
            structs::{
                CompareOp, CompareOperand, GDValue, Group, Item, ItemType, Op, RoundMode, SignMode,
            },
        },
    },
    core::b64_decode,
};

pub fn decompile_gmd(args: &Args) {
    let g = args.infile.clone().unwrap();
    let data = match GDLevel::from_gmd(&g) {
        Ok(level) => match level.get_decrypted_data() {
            Some(data) => data.objects,
            None => {
                println!("Could not parse level data.");
                return;
            }
        },
        Err(_) => {
            // try to parse as raw object string
            let file = match read_to_string(&g) {
                Ok(s) => s,
                Err(e) => {
                    println!("Could not read file: {e}");
                    return;
                }
            };
            parse_objects(&file)
        }
    };

    decomp(&data);
}

fn group_ident(id: i16, name_map: &HashMap<i16, String>) -> String {
    match name_map.get(&id) {
        Some(name) => name.clone(),
        None => format!("group_{id}"),
    }
}

fn decomp(objects: &Vec<GDObject>) {
    let mut objects = objects.clone();
    let mut groups: HashMap<i16, Vec<GDObject>> = HashMap::new();

    objects.sort_by(|a, b| a.config.pos.0.total_cmp(&b.config.pos.0));
    objects.iter().for_each(|o| {
        match groups.get_mut(&o.config.groups.get(0).unwrap_or(&Group::Regular(0)).id()) {
            Some(objs) => objs.push(o.clone()),
            None => {
                groups.insert(
                    o.config.groups.get(0).unwrap_or(&Group::Regular(0)).id(),
                    vec![o.clone()],
                );
            }
        }
    });

    let mut object_name_map: HashMap<i16, String> = HashMap::new();
    for key in groups.keys() {
        let _ = object_name_map.insert(*key, format!("group_{key}"));
    }

    for object in groups.get(&0).unwrap_or(&vec![]) {
        if object.id == TEXT && object.config.groups.is_empty() {
            let text = if let Some(GDValue::String(s)) = object.get_property(BASE64ENCODED_TEXT) {
                String::from_utf8_lossy_owned(b64_decode(s).unwrap_or(vec![]))
            } else {
                continue;
            };
            let mut iter = text.split(": ");
            if let Ok(g) = iter.next().unwrap().parse::<i16>() {
                object_name_map.insert(g, iter.collect::<Vec<&str>>().join(":"));
            }
        }
    }

    let mut g = groups.into_iter().collect::<Vec<(i16, Vec<GDObject>)>>();
    g.sort_by_key(|k| k.0);
    for (gid, objects) in g {
        println!("{}:", group_ident(gid, &object_name_map));
        for obj in objects {
            let obj_expr = match obj.id {
                ITEM_EDIT_TRIGGER => get_item_edit_expr(&from_item_edit_object(&obj).unwrap()),
                ITEM_COMPARE_TRIGGER => get_item_compare_expr(
                    &ItemCompareTrigger::from_object(&obj).unwrap(),
                    &object_name_map,
                ),
                SPAWN_TRIGGER => get_spawn_trigger_expr(
                    &SpawnTrigger::from_object(&obj).unwrap(),
                    &object_name_map,
                ),
                TEXT => {
                    // get object text (base-64 encoded)
                    let bytes = b64_decode(
                        if let GDValue::String(s) = obj
                            .get_property(BASE64ENCODED_TEXT)
                            .unwrap_or(GDValue::String(String::new()))
                        {
                            s
                        } else {
                            String::new()
                        },
                    )
                    .unwrap_or(vec![]);
                    let str = String::from_utf8_lossy(&bytes[..]);
                    format!("; Label: {str}")
                }
                n => format!("<unknown object {n}>"),
            };

            println!("    {obj_expr}");
        }
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

fn get_item_edit_expr(trg: &ItemEditTrigger) -> String {
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

fn get_item_compare_expr(trg: &ItemCompareTrigger, name_map: &HashMap<i16, String>) -> String {
    let format_side = |c: CompareOperand| -> String {
        let mut item = if c.operand_item == Item::Counter(0) {
            c.modifier.to_string()
        } else {
            let mut item = item_str(c.operand_item);
            match c.mod_op {
                Op::Mul | Op::Div if c.modifier == 1.0 => {}
                Op::Add | Op::Sub if c.modifier == 0.0 => {}
                o => item += &format!(" {} {}", op_to_str(o), c.modifier),
            }
            item
        };

        match c.rounding {
            RoundMode::None => {}
            r => item = format!("{r:?}({item})"),
        }
        match c.sign {
            SignMode::None => {}
            s => item = format!("{s:?}({item})"),
        }

        item
    };

    let comp_str = format!(
        "{} {} {}",
        format_side(trg.lhs),
        compare_op_to_str(trg.compare_op),
        format_side(trg.rhs),
    );
    let mut conf_str = String::new();

    if trg.true_id != 0 {
        conf_str += &format!("true: {} ", group_ident(trg.true_id, name_map));
    }
    if trg.false_id != 0 {
        conf_str += &format!("false: {} ", group_ident(trg.false_id, name_map));
    }
    if trg.tolerance != 0.0 {
        conf_str += &format!("tolerance: {} ", trg.tolerance);
    }

    format!("{comp_str} [{conf_str}]")
}

fn get_spawn_trigger_expr(trg: &SpawnTrigger, name_map: &HashMap<i16, String>) -> String {
    let mut conf_str = String::new();
    if trg.reset_remap {
        conf_str += "noremap "
    }
    if trg.delay != 0.0 {
        conf_str += &format!("delay: {} ± {}", trg.delay, trg.delay_variation)
    }

    format!(
        "Spawn {} {} {} | remap: {{ {}}}",
        group_ident(trg.spawn_id, name_map),
        if trg.spawn_ordered { "ordered" } else { "" },
        if conf_str.len() == 0 {
            String::new()
        } else {
            format!("[{conf_str}]")
        },
        trg.spawn_remaps
            .iter()
            .map(|(k, v)| format!("{k} = {v}, "))
            .collect::<String>()
    )
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
fn compare_op_to_str(o: CompareOp) -> &'static str {
    match o {
        CompareOp::Equals => "==",
        CompareOp::NotEquals => "!=",
        CompareOp::Less => "< ",
        CompareOp::LessOrEquals => "<=",
        CompareOp::Greater => "> ",
        CompareOp::GreaterOrEquals => ">=",
    }
}

fn from_item_edit_object(obj: &GDObject) -> Option<ItemEditTrigger> {
    if obj.id != ITEM_EDIT_TRIGGER {
        return None;
    }

    // an operand is only present if its ID != 0.
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

    // target is required, so fall back to a default rather than returning None.
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

    // transmutate COMPARE_OPERATOR since this property has type of ItemCompareOperator (we want ItemEditOperator).
    let multiply_mod = match obj.get_property(COMPARE_OPERATOR) {
        Some(GDValue::ItemCompareOperator(o)) if o == CompareOp::Less => true, // Op::Mul
        Some(GDValue::ItemCompareOperator(o)) if o == CompareOp::LessOrEquals => false, // Op::Div
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
