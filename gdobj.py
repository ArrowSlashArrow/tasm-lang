from parser import *
from rich import pretty
used_extra_objects = 0
used_extra_groups = 0
malloc_count = 0
pointer_group = 0
write_group = 0
read_group = 0
reset_block = 0
io_blocks = []
starting_counter = 0
spawn_ordered_enabled = True
spawn_delay_enabled = True
bit_packing_enabled = True
timewarp_trigger = False
memory_block_pos = [45, 165]
memory_size = 0
MEMREG = 9998
PTRPOS = 9999
params = {
    "Object id": get_obj_id,
    "Object x": get_x,
    "Object y": get_y,
    "Object x scale": get_xscale,
    "Object y scale": get_yscale,
    "Object group(s)": get_groups,
    "Z order": get_z_order,
    "Z layer": get_z_order,
    "Is touch triggered?": get_touch_triggered,
    "Is spawn triggered?": get_spawn_triggered,
    "Multi-triggerable?": get_multi_triggered,
    "Active trigger?": active_trigger,
    "Object specifics": get_obj_specs
}

# used when reading the level (only to display the contents of an object)
class GDObject():
    def __init__(self, params):
        self.params = params

    def add_param(self, param, value):
        self.params[param] = value

    def __repr__(self):
        string = ""
        for property, getter in params.items():
            string += f" - {property}: {getter(self.params)}\n"
        return string
    
    def print_raw_params(self):
        pretty.pprint(self.params, expand_all=True)


# to be clear, this is the raw api.
# on the todo list there is a clean api, but these functions 
# are the ones that have all the options to do stuff
# eventually these will be used to get the obj strings from the str groups
# and those will be strung together to yield the grand level string
# and that will be put in the level

################## TRIGGER STRING GENERATORS

# matches formatting
def counter_object_str(
    x: int, # i32
    y: int, # i32
    xscale: float, # f32
    yscale: float, # f32
    angle: float, # f32
    groups: list[int], # Vec<u16>
    ID: int, # u16
    timer: bool,
    align: int, # 0..2: center / left / right
    secondsOnly: bool,
    specialMode: int # -3..0: attempts / points / maintime / none
):
    string = f"1,1615,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,1"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    

    # specifics
    if ID > 0:
        string += f",80,{ID}"
    if secondsOnly:
        string += ",389,1"
    if specialMode < 0:
        string += f",390,{specialMode}"
    if align > 0:
        string += f",391,{align}"
    if timer:
        string += ",466,1"
    
    return string + ";"

# matches formatting
def spawn_trigger_str(
    x: int, # i32
    y: int, # i32
    xscale: float, # f32
    yscale: float, # f32
    angle: float, # f32
    groups: list[int], # Vec<u16>
    spawnTriggered: bool,
    touchTriggered: bool,
    multiTriggerable: bool,
    spawnID: int, # u16
    delay: float, # time in seconds 
    delayVar: float, # time in seconds
    resetRemap: bool, 
    spawnOrdered: bool,
    previewDisable: bool,
):
    string = f"1,1268,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,1"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    if spawnTriggered:
        string += ",62,1"
    if touchTriggered:
        string += ",11,1"
    if multiTriggerable:
        string += ",87,1"
    
    string += ",36,1"  # active trigger param

    # specifics
    if spawnID:
        string += f",51,{spawnID}"
    if delay and spawn_delay_enabled:
        string += f",63,{delay}"
    if delayVar:
        string += f",556,{delayVar}"
    
    if previewDisable:
        string += ",102,1"
    if spawnOrdered and spawn_ordered_enabled:
        string += ",441,1"
    if resetRemap:
        string += ",581,1"
    
    return string + ";"

# matches formatting
def persistent_trigger_str(
    x: int, # i32
    y: int, # i32
    xscale: float, # f32
    yscale: float, # f32
    angle: float, # f32
    groups: list[int], # Vec<u16>
    spawnTriggered: bool,
    touchTriggered: bool,
    multiTriggerable: bool,
    itemID: int, # u16
    timer: bool,  
    persistent: bool,
    targetAll: bool,
    reset: bool
):
    string = f"1,3641,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,1"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    if spawnTriggered:
        string += ",62,1"
    if touchTriggered:
        string += ",11,1"
    if multiTriggerable:
        string += ",87,1"
    
    string += ",36,1"  # active trigger param

    # specifics
    if itemID:
        string += f",80,{itemID}"
    
    if persistent:
        string += f",491,1"
    if targetAll:
        string += f",492,1"
    if reset:
        string += f",493,1"
    if timer:
        string += f",494,1"
    
    return string + ";"

# matches formatting
def compare_trigger_str(
    x: int,               # i32
    y: int,               # i32
    xscale: float,        # f32
    yscale: float,        # f32
    angle: float,         # f32
    groups: list[int],    # Vec<u16>
    spawnTriggered: bool,
    touchTriggered: bool,
    multiTriggerable: bool,
    trueID: int,          # u16
    falseID: int,         # u16
    LeftItemID: int,      # u16
    RightItemID: int,     # u16
    LeftItemType: int,    # 1..5: ["counter", "timer", "points", "maintime", "attempts"]
    RightItemType: int,   # 1..5: ["counter", "timer", "points", "maintime", "attempts"]
    LeftMod: float,       # f32
    RightMod: float,      # f32
    LeftOperator: int,    # 1..4: "+-*/"
    RightOperator: int,   # 1..4: "+-*/"
    compareOperator: int, # 0..5: ["==", ">", ">=", "<", "<=", "!="]
    tolerance: float,     # f32
    LeftRoundMode: int,   # 0..3: ["None", "Round", "Floor", "Ceiling"]
    RightRoundMode: int,  # 0..3: ["None", "Round", "Floor", "Ceiling"]
    LeftSignMode: int,    # 0..2: ["None", "Absolute", "Negative"]
    RightSignMode: int    # 0..2: ["None", "Absolute", "Negative"]
):
    string = f"1,3620,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,1"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    if spawnTriggered:
        string += ",62,1"
    if touchTriggered:
        string += ",11,1"
    if multiTriggerable:
        string += ",87,1"
    
    string += ",36,1"  # active trigger param

    # specifics
    if LeftItemID:
        string += f",80,{LeftItemID}"
    if RightItemID:
        string += f",95,{RightItemID}"

    if trueID:
        string += f",51,{trueID}"
    if falseID:
        string += f",71,{falseID}"
    
    string += f",476,{LeftItemType}"
    string += f",477,{RightItemType}"
    
    if LeftMod:
        string += f",479,{LeftMod}"
    if RightMod:
        string += f",483,{RightMod}"

    string += f",480,{LeftOperator}"
    string += f",481,{RightOperator}"
    
    if compareOperator:
        string += f",482,{compareOperator}"
    if tolerance:
        string += f",484,{tolerance}"
    
    if LeftRoundMode:
        string += f",485,{LeftRoundMode}"
    if RightRoundMode:
        string += f",486,{RightRoundMode}"
    if LeftSignMode:
        string += f",578,{LeftSignMode}"
    if RightSignMode:
        string += f",579,{RightSignMode}"

    return string + ";"

# matches formatting
def item_edit_trigger_str(
    x: int,               # i32
    y: int,               # i32
    xscale: float,        # f32
    yscale: float,        # f32
    angle: float,         # f32
    groups: list[int],    # Vec<u16>
    spawnTriggered: bool,
    touchTriggered: bool,
    multiTriggerable: bool,
    Item1ID: int,         # u16
    Item2ID: int,         # u16
    Item1Type: int,       # 1..5: ["counter", "timer", "points", "maintime", "attempts"]
    Item2Type: int,       # 1..5: ["counter", "timer", "points", "maintime", "attempts"]
    ResultID: int,        # u16
    ResultType: int,      # 1..3: ["counter", "timer", "points"]
    Mod: float,           # f32
    AssignOperator: int,  # 0..4: "=+-*/"
    ModOperator: int,     # 3..4: "*/"
    IDOperator: int,      # 1..4: "+-*/"
    IDRoundMode: int,     # 0..3: ["None", "Round", "Floor", "Ceiling"]
    AllRoundMode: int,    # 0..3: ["None", "Round", "Floor", "Ceiling"]
    IDSignMode: int,      # 0..2: ["None", "Absolute", "Negative"]
    AllSignMode: int      # 0..2: ["None", "Absolute", "Negative"]
):
    string = f"1,3619,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,1"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    if spawnTriggered:
        string += ",62,1"
    if touchTriggered:
        string += ",11,1"
    if multiTriggerable:
        string += ",87,1"
    
    string += ",36,1"  # active trigger param

    # specifics
    if Item1ID:
        string += f",80,{Item1ID}"
    if Item2ID:
        string += f",95,{Item2ID}"
    if Item1Type:
        string += f",476,{Item1Type}"
    if Item2Type:
        string += f",477,{Item2Type}"
    string += f",478,{ResultType}"
    if ResultID:
        string += f",51,{ResultID}"
    string += f",479,{Mod}"
    if AssignOperator:
        string += f",480,{AssignOperator}"
    string += f",481,{IDOperator},482,{ModOperator}"
    if IDRoundMode:
        string += f",485,{IDRoundMode}"
    if AllRoundMode:
        string += f",486,{AllRoundMode}"
    if IDSignMode:
        string += f",578,{IDSignMode}"
    if AllSignMode:
        string += f",579,{AllSignMode}"

    return string + ";"

# matches formatting
def text_object_str(
    x: int,               # i32
    y: int,               # i32
    xscale: float,        # f32
    yscale: float,        # f32
    angle: float,         # f32
    groups: list[int],    # Vec<u16>
    text: str,            # &str
    kerning: int          # i32
):
    string = f"1,914,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,1"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    
    # specifics
    string += f",24,9,31,{base64.b64encode(text.encode("utf-8")).decode()}"
    if kerning:
        string += f",488,{kerning}"
    return string + ";"

# matches formatting
def stop_trigger_str(
    x: int, # i32
    y: int, # i32
    xscale: float, # f32
    yscale: float, # f32
    angle: float, # f32
    groups: list[int], # Vec<u16>
    spawnTriggered: bool,
    touchTriggered: bool,
    multiTriggerable: bool,
    spawnID: int, # u16
    stopMode: int, # ["stop", "pause", "resume"] 
    controlID: bool, # time in seconds
):
    string = f"1,1616,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,1"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    if spawnTriggered:
        string += ",62,1"
    if touchTriggered:
        string += ",11,1"
    if multiTriggerable:
        string += ",87,1"
    
    string += ",36,1"  # active trigger param

    if spawnID:
        string += f",51,{spawnID}"
    if controlID:
        string += ",535,1"
    if stopMode:
        string += f",580,{stopMode}"

    return string + ";"

# matches formatting
def collision_block_str(
    x: int, # i32
    y: int, # i32
    xscale: float, # f32
    yscale: float, # f32
    angle: float, # f32
    groups: list[int], # Vec<u16>
    BlockID: int, # u16
    dynamicBlock: bool
):
    string = f"1,1816,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,2"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    
    string += ",36,1"  # active trigger param

    # specifics
    if BlockID:
        string += f",80,{BlockID}"
    if dynamicBlock:
        string += ",94,1"
    
    return string + ";"

# matches formatting
def collision_trigger_str(
    x: int, # i32
    y: int, # i32
    xscale: float, # f32
    yscale: float, # f32
    angle: float, # f32
    groups: list[int], # Vec<u16>
    BlockAID: int,
    BlockBID: int,
    TargetID: int,
    ActivateGroup: bool
):
    string = f"1,1815,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,2"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    
    string += ",87,1,36,1"  # multi + active trigger param

    # specifics
    if TargetID:
        string += f",51,{TargetID}"
    
    string += ",10,0.5"  # mysterious property
    if ActivateGroup:
        string += f",56,1"
    if BlockAID:
        string += f",80,{BlockAID}"
    if BlockBID:
        string += f",95,{BlockBID}"
    
    return string + ";"

# matches formatting
def toggle_trigger_str(
    x: int, # i32
    y: int, # i32
    xscale: float, # f32
    yscale: float, # f32
    angle: float, # f32
    groups: list[int], # Vec<u16>
    spawnTriggered: bool,
    touchTriggered: bool,
    multiTriggerable: bool,
    TargetID: int,
    ActivateGroup: bool
):
    string = f"1,1049,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,2"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    if spawnTriggered:
        string += ",62,1"
    if touchTriggered:
        string += ",11,1"
    if multiTriggerable:
        string += ",87,1"
        
    string += ",87,1,36,1"  # multi + active trigger param

    # specifics
    # fun fact: this is just a collision trigger without all the block stuff (and the mystery property)
    if TargetID:
        string += f",51,{TargetID}"
    
    if ActivateGroup:
        string += f",56,1"
    
    return string + ";"

# matches formatting
def move_trigger_str(
    x: int, # i32
    y: int, # i32
    xscale: float, # f32
    yscale: float, # f32
    angle: float, # f32
    groups: list[int], # Vec<u16>
    spawnTriggered: bool,
    touchTriggered: bool,
    multiTriggerable: bool,
    dX: int,
    dY: int,
    time: float,
    target: int,
    targetMode: bool = False,
    aim: int = 0
):
    string = f"1,901,2,{x},3,{y},64,1,67,1" # dont fade, dont enter
    if groups:
        string += ",57," + ".".join([str(g) for g in groups])
    string += ",155,1"
    if angle != 0:
        string += f",6,{angle}"
    if xscale != 1:
        string += f",128,{xscale}"
    if yscale != 1:
        string += f",129,{yscale}"
    if spawnTriggered:
        string += ",62,1"
    if touchTriggered:
        string += ",11,1"
    if multiTriggerable:
        string += ",87,1"
        
    # specifics
    if targetMode:
        string += f",28,0"
        string += f",29,0"
        if time:
            string += f",10,{time}"
        string += f",30,0,85,2,71,{aim},100,1"
        if target:
            string += f",51,{target}"
        
    else:
        string += f",28,{dX}"
        string += f",29,{dY}"
        if time:
            string += f",10,{time}"
        if target:
            string += f",51,{target}"
    
    return string + ";"
    
################## INIT

def make_persistent(item, **kwargs):
    ypos = kwargs["group"] * 30 + 75
    pref, *id = item
    id = int("".join(id))
    return persistent_trigger_str(
        -45,
        ypos,
        1, 1, 0, [],
        False, False, False,
        id,
        pref.lower() == "t",
        True,
        False,
        False
    )

def display_item_pos(item, pos, **kwargs):
    ypos = float(pos) * 30 + 75
    itemtype, id = unpack_item(item)
    return counter_object_str(
        -105, ypos, 0.5, 0.5, 0, [], id, itemtype == 2,
        0, False, 0
    )

def display_item(item, **kwargs):
    ypos = float(kwargs["index"]) * 30 + 45
    itemtype, id = unpack_item(item)
    return counter_object_str(
        -105, ypos, 0.5, 0.5, 0, [], id, itemtype == 2,
        0, False, 0
    )

################## MOVE

def mov_num(item, number, **kwargs):
    global used_extra_objects
    xpos, ypos, group = unpack_kwargs(**kwargs)
    dX = 1 if kwargs["squish"] else 30
    itemtype, id = unpack_item(item)
    number = int(number)
    # bitpacker
    if number > 16777216 and bit_packing_enabled:
        used_extra_objects += 2
        big = number // 65536
        small = number % 65536
        return item_edit_trigger_str(
            xpos, ypos, 1, 1, 0, [group],
            True, False, True,  # ItemID1, ...
            0, 0, 0, 0,
            id,  # result id
            itemtype,  # result type
            big,  # number
            0, 3, 1, 0, 0, 0, 0
        ) + item_edit_trigger_str(
            xpos + dX, ypos, 1, 1, 0, [group],
            True, False, True,  # ItemID1, ...
            0, 0, 0, 0,
            id,  # result id
            itemtype,  # result type
            65536,  # number
            3, 3, 1, 0, 0, 0, 0
        ) + item_edit_trigger_str(
            xpos + dX, ypos, 1, 1, 0, [group],
            True, False, True,  # ItemID1, ...
            0, 0, 0, 0,
            id,  # result id
            itemtype,  # result type
            small,  # number
            1, 3, 1, 0, 0, 0, 0
        )
    else:
        return item_edit_trigger_str(
            xpos, ypos, 1, 1, 0, [group],
            True, False, True,  # ItemID1, ...
            0, 0, 0, 0,
            id,  # result id
            itemtype,  # result type
            number,  # number
            0, 3, 1, 0, 0, 0, 0
        )

################## ARITHMETIC

def arithmetic_2counters(result, item1, operator, **kwargs):
    xpos, ypos, group = unpack_kwargs(**kwargs)
    result_itemtype, result_id = unpack_item(result)
    original_itemtype, original_id = unpack_item(item1)

    return item_edit_trigger_str(
        xpos, ypos, 1, 1, 0, [group], True, False, True,
        original_id, 0, original_itemtype, 1,
        result_id, result_itemtype,
        1, min(operator, 4), 3, 1, 0, 0, int(operator == 5) * 2, 0
    )
    
def arithmetic_2counters_num(result, item1, mod, operator, **kwargs):
    xpos, ypos, group = unpack_kwargs(**kwargs)
    result_itemtype, result_id = unpack_item(result)
    original_itemtype, original_id = unpack_item(item1)

    return item_edit_trigger_str(
        xpos, ypos, 1, 1, 0, [group], True, False, True,
        original_id, 0, original_itemtype, 1,
        result_id, result_itemtype,
        mod, min(operator, 4), 3, 1, 0, 0, int(operator == 5) * 2, 0
    )
    
def arithmetic_counter_num(result, num, operator, **kwargs):
    xpos, ypos, group = unpack_kwargs(**kwargs)
    result_itemtype, result_id = unpack_item(result)

    return item_edit_trigger_str(
        xpos, ypos, 1, 1, 0, [group], True, False, True,
        0, 0, 0, 0, result_id, result_itemtype, num,
        min(operator, 4), 3, 1, 0, 0, int(operator == 5) * 2, 0
    )

def arithmetic_3counters(result, item1, item2, operator, **kwargs):
    xpos, ypos, group = unpack_kwargs(**kwargs)
    result_itemtype, result_id = unpack_item(result)
    item1_itemtype, item1_id = unpack_item(item1)
    item2_itemtype, item2_id = unpack_item(item2)

    return item_edit_trigger_str(
        xpos, ypos, 1, 1, 0, [group], True, False, True,
        item1_id, item2_id, item1_itemtype, item2_itemtype,
        result_id, result_itemtype,
        1, min(operator, 4), 3, 1, 0, 0, int(operator == 5) * 2, 0
    )
    

def mov_counter(*args, **kwargs):
    return arithmetic_2counters(*args, 0, **kwargs)
    
def add_num(*args, **kwargs):
    return arithmetic_counter_num(*args, 1, **kwargs)

def add_counter(*args, **kwargs):
    return arithmetic_2counters(*args, 1, **kwargs)

def add2(*args, **kwargs):
    return arithmetic_3counters(*args, 1, **kwargs)
    
def sub_num(*args, **kwargs):
    return arithmetic_counter_num(*args, 2, **kwargs)

def sub_counter(*args, **kwargs):
    return arithmetic_2counters(*args, 2, **kwargs)

def sub2(*args, **kwargs):
    return arithmetic_3counters(*args, 2, **kwargs)

def mul_num(*args, **kwargs):
    return arithmetic_counter_num(*args, 3, **kwargs)

def mul_counter(*args, **kwargs):
    return arithmetic_2counters(*args, 3, **kwargs)

def mul2(*args, **kwargs):
    return arithmetic_3counters(*args, 3, **kwargs)

def mul2num(*args, **kwargs):
    return arithmetic_3counters(*args, 3, **kwargs)

def div_num(*args, **kwargs):
    return arithmetic_counter_num(*args, 4, **kwargs)

def div_counter(*args, **kwargs):
    return arithmetic_2counters(*args, 4, **kwargs)

def div2(*args, **kwargs):
    return arithmetic_3counters(*args, 4, **kwargs)

def div2num(*args, **kwargs):
    return arithmetic_3counters(*args, 4, **kwargs)

def fldiv_num(*args, **kwargs):
    return arithmetic_counter_num(*args, 5, **kwargs)

def fldiv_counter(*args, **kwargs):
    return arithmetic_2counters(*args, 5, **kwargs)

def fldiv2(*args, **kwargs):
    return arithmetic_3counters(*args, 5, **kwargs)

def fldiv2num(*args, **kwargs):
    return arithmetic_3counters(*args, 5, **kwargs)


################## ITEM COMPARE

def spawn_item(trueID, item1, item2, operator, **kwargs):
    global used_extra_groups
    used_extra_groups = 1
    
    # init some values
    xpos, ypos, group = unpack_kwargs(**kwargs)
    first_itemtype, first_id = unpack_item(item1) 
    second_itemtype, second_id = unpack_item(item2)
    
    nextfree = kwargs["nextfree"]
    needs_spawn = kwargs["lengths"][trueID] > 1
    
    spawn_trigger = ""
    compare_truegroup = trueID
    if needs_spawn:
        spawn_trigger = spawn_trigger_str(
            xpos, ypos - 7.5, 1, 0.5, 0, [nextfree], True, False, True, 
            trueID, 0.0042, 0, False, True, False  # 0.0042 = 1/240
        )
        compare_truegroup = nextfree
        used_extra_groups += 1
    
    
    # then the triggers
    return compare_trigger_str(
        xpos, ypos + 7.5, 1, 0.5, 0, [group], True, False, True, 
        compare_truegroup, 0, first_id, second_id, first_itemtype, second_itemtype, 1, 1,
        3, 3, operator, 0, 0, 0, 0, 0
    ) + spawn_trigger
    
def spawn_num(trueID, item1, num, operator, **kwargs):
    global used_extra_groups
    
    # init some values
    xpos, ypos, group = unpack_kwargs(**kwargs)
    first_itemtype, first_id = unpack_item(item1) 
    nextfree = kwargs["nextfree"]
    needs_spawn = kwargs["lengths"][trueID] > 1
    
    spawn_trigger = ""
    compare_truegroup = trueID
    if needs_spawn:
        spawn_trigger = spawn_trigger_str(
            xpos, ypos - 7.5, 1, 0.5, 0, [nextfree], True, False, True, 
            trueID, 0.0042, 0, False, True, False  # 0.0042 = 1/240
        )
        compare_truegroup = nextfree
        used_extra_groups += 1
    
    # then the triggers
    return compare_trigger_str(
        xpos, ypos + 7.5, 1, 0.5, 0, [group], True, False, True, 
        compare_truegroup, 0, first_id, 0, first_itemtype, 1, 1, float(num),
        3, 3, operator, 0, 0, 0, 0, 0
    ) + spawn_trigger


def spawn_equals_item(*args, **kwargs):
    return spawn_item(*args, 0, **kwargs)

def spawn_equals_num(*args, **kwargs):
    return spawn_num(*args, 0, **kwargs)

def spawn_greater_item(*args, **kwargs):
    return spawn_item(*args, 1, **kwargs)

def spawn_greater_num(*args, **kwargs):
    return spawn_num(*args, 1, **kwargs)

def spawn_gequals_item(*args, **kwargs):
    return spawn_item(*args, 2, **kwargs)

def spawn_gequals_num(*args, **kwargs):
    return spawn_num(*args, 2, **kwargs)

def spawn_less_item(*args, **kwargs):
    return spawn_item(*args, 3, **kwargs)

def spawn_less_num(*args, **kwargs):
    return spawn_num(*args, 3, **kwargs)

def spawn_lequals_item(*args, **kwargs):
    return spawn_item(*args, 4, **kwargs)

def spawn_lequals_num(*args, **kwargs):
    return spawn_num(*args, 4, **kwargs)

def spawn_nequals_item(*args, **kwargs):
    return spawn_item(*args, 5, **kwargs)

def spawn_nequals_num(*args, **kwargs):
    return spawn_num(*args, 5, **kwargs)


def fork_item(trueID, falseID, item1, item2, operator, **kwargs):
    global used_extra_groups
    
    # unpack values
    xpos, ypos, group = unpack_kwargs(**kwargs)
    nextfree = kwargs["nextfree"]
    
    first_itemtype, first_id = unpack_item(item1) 
    second_itemtype, second_id = unpack_item(item2)
    
    true_needs_spawn = kwargs["lengths"][trueID] > 1
    false_needs_spawn = kwargs["lengths"][falseID] > 1
    
    compare_truegroup = trueID
    spawn_trigger = ""
    if true_needs_spawn:
        compare_truegroup = nextfree
        spawn_trigger = spawn_trigger_str(
            xpos, ypos + 10, 1, 0.3, 0, [nextfree], True, False, True, 
            trueID, 0.0042, 0, False, True, False  # 0.0042 = 1/240
        ) 
        used_extra_groups += 1
        
    compare_falsegroup = falseID
    second_spawn_trigger = ""
    if false_needs_spawn:
        compare_falsegroup = nextfree + used_extra_groups
        second_spawn_trigger = spawn_trigger_str(
            xpos, ypos - 10, 1, 0.3, 0, [nextfree + used_extra_groups], True, False, True, 
            falseID, 0.0042, 0, False, True, False  # 0.0042 = 1/240
        ) 
        used_extra_groups += 1
    
    item_compare_str = compare_trigger_str(
        xpos, ypos, 1, 0.3, 0, [group], True, False, True, 
        compare_truegroup, 
        compare_falsegroup, 
        first_id, second_id, first_itemtype, second_itemtype, 1, 1,
        3, 3, operator,
        0, 0, 0, 0, 0
    )
    
    return item_compare_str + spawn_trigger + second_spawn_trigger
    
def fork_num(trueID, falseID, item1, num, operator, **kwargs):
    global used_extra_groups
    
    # unpack values
    xpos, ypos, group = unpack_kwargs(**kwargs)
    nextfree = kwargs["nextfree"]
    first_itemtype, first_id = unpack_item(item1) 
    
    true_needs_spawn = kwargs["lengths"][trueID] > 1
    false_needs_spawn = kwargs["lengths"][falseID] > 1
    
    compare_truegroup = trueID
    spawn_trigger = ""
    if true_needs_spawn:
        compare_truegroup = nextfree
        spawn_trigger = spawn_trigger_str(
            xpos, ypos + 10, 1, 0.3, 0, [nextfree], True, False, True, 
            trueID, 0.0042, 0, False, True, False  # 0.0042 = 1/240
        ) 
        used_extra_groups += 1
        
    compare_falsegroup = falseID
    second_spawn_trigger = ""
    if false_needs_spawn:
        compare_falsegroup = nextfree + used_extra_groups
        second_spawn_trigger = spawn_trigger_str(
            xpos, ypos + 10, 1, 0.3, 0, [nextfree + used_extra_groups], True, False, True, 
            trueID, 0.0042, 0, False, True, False  # 0.0042 = 1/240
        ) 
        used_extra_groups += 1
    
    item_compare_str = compare_trigger_str(
        xpos, ypos, 1, 0.3, 0, [group], True, False, True, 
        compare_truegroup, 
        compare_falsegroup, 
        first_id, 0, first_itemtype, 1, 1, float(num),
        3, 3, operator,
        0, 0, 0, 0, 0
    )
    
    return item_compare_str + spawn_trigger + second_spawn_trigger

def fork_equals_item(*args, **kwargs):
    return fork_item(*args, 0, **kwargs)

def fork_equals_num(*args, **kwargs):
    return fork_num(*args, 0, **kwargs)

def fork_greater_item(*args, **kwargs):
    return fork_item(*args, 1, **kwargs)

def fork_greater_num(*args, **kwargs):
    return fork_num(*args, 1, **kwargs)

def fork_gequals_item(*args, **kwargs):
    return fork_item(*args, 2, **kwargs)

def fork_gequals_num(*args, **kwargs):
    return fork_num(*args, 2, **kwargs)

def fork_less_item(*args, **kwargs):
    return fork_item(*args, 3, **kwargs)

def fork_less_num(*args, **kwargs):
    return fork_num(*args, 3, **kwargs)

def fork_lequals_item(*args, **kwargs):
    return fork_item(*args, 4, **kwargs)

def fork_lequals_num(*args, **kwargs):
    return fork_num(*args, 4, **kwargs)

def fork_nequals_item(*args, **kwargs):
    return fork_item(*args, 5, **kwargs)

def fork_nequals_num(*args, **kwargs):
    return fork_num(*args, 5, **kwargs)

################## THREADING / CONTROL FLOW

def spawn_group(spawn_group, **kwargs):
    xpos, ypos, group = unpack_kwargs(**kwargs)
    return spawn_trigger_str(
        xpos, ypos, 1, 1, 0, [group], True, False, True, 
        spawn_group, 0.0042, 0, False, True, False  # 0.0042 = 1/240
    )

def nop(**kwargs):
    return ""

################## MEMORY

def initmem(numbers, **kwargs):
    def bitpack(idx, num):
        out_str = ""
        if num <= 16777216 or not bit_packing_enabled:
            out_str = item_edit_trigger_str(
                memory_block_pos[0] - 63.75, y_offset + 7.5 * (idx + 1) - 18.75, 0.25, 0.25, 0, [], False, False, False,
                0, 0, 1, 1, starting_counter + idx, 1, num, 0, 3, 1, 0, 0, 0, 0
            )
        else:
            # bitpack
            big, small = num // 65536, num % 65536
            out_str = item_edit_trigger_str(
                memory_block_pos[0] - 63.75, y_offset + 7.5 * (idx + 1) - 18.75, 0.25, 0.25, 0, [], False, False, False,
                0, 0, 1, 1, starting_counter + idx, 1, big, 0, 3, 1, 0, 0, 0, 0
            ) + item_edit_trigger_str(
                memory_block_pos[0] - 56.25, y_offset + 7.5 * (idx + 1) - 18.75, 0.25, 0.25, 0, [], False, False, False,
                0, 0, 1, 1, starting_counter + idx, 1, 65536, 3, 3, 1, 0, 0, 0, 0
            ) + item_edit_trigger_str(
                memory_block_pos[0] - 48.75, y_offset + 7.5 * (idx + 1) - 18.75, 0.25, 0.25, 0, [], False, False, False,
                0, 0, 1, 1, starting_counter + idx, 1, small, 1, 3, 1, 0, 0, 0, 0
            )
        return out_str
    nums = [int(x) for x in numbers.split(",")]
    y_offset = memory_block_pos[1] + kwargs["subroutine_count"] * 30
    return "".join(
        [bitpack(idx, num) for idx, num in enumerate(nums)]
    )

def malloc(amount, **kwargs):
    # uses 4 + 4amount groups
    global malloc_count, used_extra_groups, pointer_group, read_group, write_group, reset_block, starting_counter, memory_size
    memory_size = amount
    
    x_offset = memory_block_pos[0]
    y_offset = memory_block_pos[1] + kwargs["subroutine_count"] * 30
    
    amount = int(amount)
    starting_counter = MEMREG - amount
    
    # block ids (padding and pointer)
    LEFTID = 9997
    RIGHTID = 9998
    POINTERID = 9999
    
    # you CANNOT have malloc more than once
    malloc_count += 1
    if malloc_count > 1:
        return ""
    
    out_str = ""
    nextfree = kwargs["nextfree"]
    reset_block = nextfree
    # add reset object (where the pointer block goes when you call RPTR)
    out_str += f"1,1,2,{x_offset},3,{y_offset - 30},128,0.5,129,0.5,57,{nextfree};"
    # pointer block
    out_str += collision_block_str(
        x_offset, y_offset - 30, 0.8, 0.8, 0, [nextfree + 1], POINTERID, True
    )
    pointer_group = nextfree + 1
    
    nextfree += 1
    read_group = nextfree + 1 # toggled on when in read mode
    write_group = nextfree + 2  # toggled on when in write mode
    used_extra_groups += 3
    nextfree += 3
    # memory cells
    for idx, counter in enumerate(range(starting_counter, MEMREG)):
        item_group = nextfree
        
        xpos = idx * 30 + x_offset
        # add a memcell
        used_extra_groups += 1
        collision_block = collision_block_str(
            xpos, y_offset, 1, 1, 0, [], idx + 1, False
        )
        collision_trigger = collision_trigger_str(
            x_offset - 71.25, y_offset + (idx + 1) * 7.5 - 18.75, 0.25, 0.25, 0, [], idx + 1, POINTERID, 
            item_group, True
        ) # all collision triggers x should be < 0 to be initialised immediately
        write_item = item_edit_trigger_str(  # write register to this memory location
            xpos, y_offset + 30, 1, 1, 0, [item_group, write_group],
            True, False, True, MEMREG, 0, 1, 0, counter, 1, 1, 0, 3, 1, 0, 0, 0, 0
        )
        read_item = item_edit_trigger_str(  # write register to this memory location
            xpos, y_offset + 60, 1, 1, 0, [item_group, read_group],
            True, False, True, counter, 0, 1, 0, MEMREG, 1, 1, 0, 3, 1, 0, 0, 0, 0
        )
        counter_obj = counter_object_str(
            xpos, y_offset - 60, 0.4, 0.4, -30, [], counter, False, 0, False, 0
        )
        # this is here so that extra time isnt occupied on the group that moved the pointer here
        pos_reset = move_trigger_str(
            xpos, y_offset + 90, 1, 1, 0, [item_group], True, False, True, 0, -30, 0, pointer_group
        ) 
        out_str += collision_block + collision_trigger + write_item + read_item + counter_obj + pos_reset
        nextfree += 1
    
    # padding blocks
    out_str += collision_block_str(
        x_offset - 75, y_offset - 30, 3.8, 0.8, 0, [], LEFTID, True
    )
    out_str += collision_block_str(
        x_offset + amount * 30 + 45, y_offset - 30, 3.8, 0.8, 0, [], RIGHTID, True
    )
    
    y_offset -= 30
    # padding triggers (you can escape them if you move them enough but idk how to fix that)
    out_str += collision_trigger_str(  # move right (left padding)
        x_offset - 60, y_offset - 22.5, 0.5, 0.5, 0, [], LEFTID, POINTERID, 
        nextfree, True
    ) + collision_trigger_str(  # move left
        x_offset - 60, y_offset - 37.5, 0.5, 0.5, 0, [], RIGHTID, POINTERID, 
        nextfree + 1, True
    ) + move_trigger_str(  # actual move right
        x_offset - 75, y_offset - 22.5, 0.5, 0.5, 0, [nextfree], True, False, True,
        30, 0, 0, pointer_group
    ) + move_trigger_str(  # actual move left
        x_offset - 75, y_offset - 37.5, 0.5, 0.5, 0, [nextfree + 1], True, False, True,
        -30, 0, 0, pointer_group
    ) + item_edit_trigger_str(  # reset pointer position (left)
        x_offset - 90, y_offset - 22.5, 0.5, 0.5, 0, [nextfree], True, False, True,
        0, 0, 1, 1, PTRPOS, 1, 0, 0, 3, 1, 0, 0, 0, 0
    ) + item_edit_trigger_str(  # reset pointer position (right)
        x_offset - 90, y_offset - 37.5, 0.5, 0.5, 0, [nextfree + 1], True, False, True,
        0, 0, 1, 1, PTRPOS, 1, amount - 1, 0, 3, 1, 0, 0, 0, 0
    )
    
    out_str += text_object_str(
        x_offset, y_offset + 150, 0.5, 0.5, 0, [], "memory", 0
    )
    
    used_extra_groups += 3
    return out_str

def mfunc(**kwargs):
    global used_extra_objects
    used_extra_objects += 2 if kwargs["squish"] else 0 # give it time to register the collision
    xpos, ypos, group = unpack_kwargs(**kwargs)
    return move_trigger_str(
        xpos, ypos, 1, 1, 0, [group], True, False, True, 0, 30, 0, pointer_group
    )
    
def switch_mem_mode(read, **kwargs):
    xpos, ypos, group = unpack_kwargs(**kwargs)
    ypos += 7.5
    return toggle_trigger_str(
        xpos, ypos, 1, 0.5, 0, [group], True, False, True, read_group, read 
    ) + toggle_trigger_str(
        xpos, ypos - 15, 1, 0.5, 0, [group], True, False, True, write_group, not read
    )
    
def mread(**kwargs):
    return switch_mem_mode(True, **kwargs)

def mwrite(**kwargs):
    return switch_mem_mode(False, **kwargs)

def mptr(amount, **kwargs):
    xpos, ypos, group = unpack_kwargs(**kwargs)
    item_edit_kwargs = kwargs
    item_edit_kwargs["yoffset"] = -7.5
    item_edit_kwargs["yscale"] = 0.5
    ypos += 7.5
    out = move_trigger_str(
        xpos, ypos, 1, 0.5, 0, [group], True, False, True, int(amount) * 30, 0, 0, pointer_group
    ) + add_num(f"C{PTRPOS}", int(amount), **item_edit_kwargs)
    
    return out

def mreset(**kwargs):
    xpos, ypos, group = unpack_kwargs(**kwargs)
    ypos += 7.5   
    item_edit_kwargs = kwargs
    item_edit_kwargs["yoffset"] = -7.5
    item_edit_kwargs["yscale"] = 0.5
    return move_trigger_str(
        xpos, ypos, 1, 0.5, 0, [group], True, False, True, 0, 0, 0, pointer_group, True, reset_block
    ) + mov_num(f"C{PTRPOS}", 0, **item_edit_kwargs)

################## I/O

def ioblock(spawngroup, position, text, **kwargs):
    global used_extra_objects
    used_extra_objects = -1  # no object is used in the group

    if position in io_blocks:
        print(f"WARNING: There is already an IOBlock at position {position}. This one will be skipped.")
        return ""
    
    io_blocks.append(position)
    xpos, ypos = 75 + int(position) * 30, 75
    return text_object_str(
        xpos, ypos, 0.25, 0.25, 0, [], str(text), 0
    ) + f"1,1,2,{xpos},3,{ypos};" + spawn_trigger_str(
        xpos, ypos, 1, 1, 0, [], False, True, True, spawngroup, 0, 0, False, True, False
    )

################## UTILS

# returns itemtype, index from itemstr
def unpack_item(item: str):
    pref, *id = item
    id = int("".join(id))
    itemtype = {
        "c": 1,
        "t": 2
    }[pref.lower()]
    return itemtype, id

# placeholder
def placeholder(*args, **kwargs):
    return ""

def reset_extra_objects():
    global used_extra_objects
    used_extra_objects = 0

def reset_extra_groups():
    global used_extra_groups
    used_extra_groups = 0

def unpack_kwargs(**kwargs):
    xpos = 105 + (1 if kwargs["squish"] else 30) * float(kwargs["index"])
    ypos = kwargs["group"] * 30 + 75
    group = kwargs["group"]
    return xpos, ypos, group

# 05/07: yes, there's a better way to do this
# 05/07: however, i can't be bothered to refactor this
# 13/07: the time has come to remove 800 lines of copy paste