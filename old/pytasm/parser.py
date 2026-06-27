import base64

objects = {
    1: "Default block",
    901: "Move trigger",
    914: "Text object",
    1049: "Toggle trigger",
    1268: "Spawn trigger",
    1615: "Counter",
    1616: "Stop trigger",
    1815: "Collision trigger",
    1816: "Collision block",
    1935: "Time warp trigger",
    3619: "Item edit trigger",
    3620: "Item compare trigger",
    3641: "Persistent item trigger"
}

_itemtype = ["counter", "timer", "points", "maintime", "attempts"]
_compare = ["==", ">", ">=", "<", "<=", "!="]
_alignments = ["center", "left", "right"]

def _bool(value):
    return ["no", "yes"][int(value)]

def item(value):
    return _itemtype[int(value)]

def roundmode(value):
    return ["None", "Round", "Floor", "Ceiling"][int(value)]

def signmode(value):
    return ["None", "Absolute", "Negative"][int(value)]

def operator(value):
    return "=+-*/"[int(value)]

def active_trigger(obj_dict):
    return _bool(obj_dict.get("36", 0))

def get_obj_id(obj_dict):
    obj = obj_dict["1"]
    return f"{obj} ({objects.get(int(obj), "Unknown")})"  

def get_x(obj_dict):
    return obj_dict["2"]

def get_y(obj_dict):
    return obj_dict["3"]

def get_xscale(obj_dict):
    return float(obj_dict.get("128", 1))

def get_yscale(obj_dict):
    return float(obj_dict.get("129", 1))

def get_groups(obj_dict):
    return ", ".join(obj_dict.get("57", "").split("."))

def get_touch_triggered(obj_dict):
    return _bool((obj_dict.get("11", 0)))

def get_spawn_triggered(obj_dict):
    return _bool((obj_dict.get("62", 0)))

def get_multi_triggered(obj_dict):
    return _bool((obj_dict.get("87", 0)))

def get_z_order(obj_dict):
    return [int(obj_dict.get("24", 5))]

def get_z_order(obj_dict):
    return int(obj_dict.get("25", 1))

def return_self(a):
    return a

def compare_itemtype(value):
    return _itemtype[int(value) - 1]

def compare_operator(value):
    return _compare[int(value)]

def get_align_str(value):
    return _alignments[int(value)]

def get_special_mode(value):
    return ['Attempts', 'Points', 'GameTime', 'No'][3 + int(value)]

def b64decode(value):
    return base64.b64decode(value).decode()

def stopmode(value):
    return ["Stop", "Pause", "Resume"][int(value)]

def get_obj_specs(obj_dict):
    # properties labelled as "X1" are for lhs, "X2" are for rhs
    specs_table = { # Object id: {property: [obj_dict index, default, function to apply after]}
        3619: {  # item edit trigger
            "ItemID1": [80, 0, return_self],
            "ItemID2": [95, 0, return_self],
            "ItemType1": [476, 0, lambda x: item(int(x) - 1)],
            "ItemType2": [477, 0, lambda x: item(int(x) - 1)],
            "ResultID": [51, 0, return_self],
            "ResultType": [478, None, item],  # always defined
            "mod": [479, None, return_self],  # always defined
            "AssignmentOperator": [480, 0, operator],
            "IDOperator": [481, None, operator],  # always defined
            "ModOperator": [482, None, operator],  # always defined
            "roundModeIDs": [485, 0, roundmode],
            "roundModeAll": [486, 0, roundmode],
            "signModeIDs": [578, 0, signmode],
            "signModeAll": [579, 0, signmode],
        },
        3620: {  # item compare trigger
            "TrueID": [51, 0, return_self],
            "FalseID": [71, 0, return_self],
            "ItemID1": [80, 0, return_self],
            "ItemID2": [95, 0, return_self],
            "ItemType1": [476, 1, compare_itemtype],
            "ItemType2": [477, 1, compare_itemtype],
            "Mod1": [479, 0, return_self],
            "Mod2": [483, 0, return_self],
            "operator1": [480, 1, operator],
            "operator2": [481, 1, operator],
            "compareOperator": [482, 0, compare_operator],
            "tolerance": [484, 0, return_self],
            "roundMode1": [485, 0, roundmode],
            "roundMode2": [486, 0, roundmode],
            "signMode1": [578, 0, signmode],
            "signMode2": [579, 0, signmode]
        },
        1615: {  # counter obj
            "ItemID": [80, 0, return_self],
            "TimeCounter": [466, 0, _bool],
            "align": [391, 0, get_align_str],
            "secondsOnly": [389, 0, _bool],
            "SpecialMode": [390, 0, get_special_mode],
        },
        1616: {  # stop trigger
            "Group": [51, 0, return_self],
            "stopMode": [580, 0, stopmode],
            "controlID": [535, 0, _bool]
        },
        1935: {  # time warp
            "scale": [120, 1, return_self]
        },
        1268: {  # spawn trigger
            "groupID": [51, 0, return_self],
            "delay": [63, 0, return_self],
            "delayVariation": [556, 0, return_self],
            "resetRemap": [581, 0, _bool],
            "spawnOrdered": [441, 0, _bool],
            "previewDisable": [102, 0, _bool],
        },
        3641: {  # persistent item trigger
            "ItemID": [80, 0, return_self],
            "TimeCounter": [494, 0, _bool], 
            "Persistence": [491, 0, _bool], 
            "TargetAll": [492, 0, _bool], 
            "Reset": [493, 0, _bool], 
        },
        914: {  # text object
            "Text": [31, None, b64decode],
            "Kerning": [488, 0, int]
        },
        1816: {  # Collision block
            "BlockID": [80, 0, return_self],
            "DynamicBlock": [94, 0, _bool]
        },
        1815: {  # Collision trigger
            "BlockA": [80, 0, return_self],
            "BlockB": [95, 0, return_self],
            "TargetID": [51, 0, return_self],
            "ActivateGroup": [56, 0, _bool]
        },
        1049: {
            "TargetID": [51, 0, return_self],
            "ActivateGroup": [56, 0, _bool]
        }
    }

    obj_id = int(obj_dict["1"])
    if obj_id not in specs_table:
        return "{}"
    
    specs = {}
    spec_list = specs_table[obj_id]
    for name, value in spec_list.items():
        specs[name] = value[2](obj_dict.get(str(value[0]), value[1]))
            
    
    return "{\n   │ " + "\n   │ ".join([f"{k}: {v}" for k, v in specs.items()]) + "\n}"
