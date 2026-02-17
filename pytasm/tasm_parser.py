from rich import pretty
import os, gdobj, commands
from gdobj import reset_extra_groups, reset_extra_objects

errors = 0
defined_routines = []
start_group = 0
round_error = False
warnings = True
memory_instructions = ["INITMEM", "MREAD", "MWRITE", "MFUNC", "MPTR", "MRESET"]  # these were outdated
instructions = commands.INSTRUCTIONS
# these are vars
ALIASES = {
    "MEMREG": f"C{gdobj.MEMREG}", 
    "PTRPOS": f"C{gdobj.PTRPOS}",
}
# these are constants
INTSTRS = ["MEMSIZE"]

def display_err_msg(line, index, err_msg, routine="", warning=False):
    global errors
    line_num_length = len(f"{index: >5}")
    if routine:
        print(f"{" " * line_num_length} | {routine}:")
    print(f"{index: >5} |     {line}")
    print(f"{" " * line_num_length} + {err_msg}\n")
    if not warning:
        errors += 1


def is_int(x: str):
    initial_check = x[1:].isdigit() if x[0] == "-" else x.isdigit()
    if not initial_check:
        return False
    
    if not (-2_147_483_648 < int(x) < 2_147_483_648):
        return False

    return True

def is_num(x: str):
    global round_error
    round_error = False
    if x in INTSTRS:
        return True
    try:
        n = float(x)
        if n < -16_777_216 or n > 16_777_216:
            round_error = True
        return True
    except:
        return False

def is_counter(x: str):
    return x[1:].isdigit() and x[0] == "C"

def is_item(x: str):
    if len(x) < 2:
        return False
    if not is_int(x[1:]):
        return False
    return 0 < int(x[1:]) < 65536 and x[0].lower() in "ct"
    
def int_array(x: str):
    try:
        [int(x) for x in x.split(",")]
        return True
    except:
        return False
    
types = {
    "int": is_int,
    "int_array": int_array,
    "str": lambda x: True,
    "number": is_num, 
    "counter": is_counter,
    "item": is_item,
    "routine": lambda x: x in defined_routines,
    "group": lambda x: x in defined_routines or is_int(x)
}


# supported types: int, number, counter
def is_type(value: str, type: str):
    if type not in types:
        print(f"Unsupported type: {type}")
        return False

    return types[type](value)    


def valid_arguments(instruction, args, line, line_index):
    # get valid argsets
    valid_argsets = instructions[instruction]["args"]
    actual_args = args.split(", ")
    # for each possible argset
    for index, argset in enumerate(valid_argsets): 
        # the part we care about is the first element   
        argset = argset[0]     
        actual_args = [arg for arg in actual_args if arg != ""]
        # skip ip if the lengths are not equal
        if len(actual_args) != len(argset):
            continue

        # match each argument to each other argument
        valid_argset = True
        for expected, actual in zip(argset, actual_args):
            
            # loop thru each argument to make sure every single one matches
            # for the argset to be considered valid
            if not is_type(actual, expected) and actual not in ALIASES:
                valid_argset = False
            
            if round_error and warnings and not gdobj.bit_packing_enabled:
                display_err_msg(line, line_index, "WARNING: GD will incorrectly round numbers above 16,777,216. This operation may result in an inaccuracy.", warning=True)
        
        if valid_argset:
            # return the index to the function that handles this specific argset
            return index + 1
    
    return 0


def validate_instruction(line, index, instruction, args, routine):
    # valid instruction args not defined in instructions dict
    if instruction not in list(instructions.keys()):
        display_err_msg(line, index, f"Invalid instruction: {instruction}")
        return False
    
    allowed_list = instructions[instruction]["allowed"]
    if allowed_list != "*" and "*" not in allowed_list:
        # check if instruction is allowed in the routine
        if routine not in allowed_list:
            display_err_msg(line, index, f"Instruction '{instruction}' not allowed in routine '{routine}'.", routine)
    
    
    # instruction exists, but are the arguments valid?
    
    idx = valid_arguments(instruction, args, line, index)
    if not idx:
        display_err_msg(line, index, f"Cannot call '{instruction}' with these arguments: {args}.", routine)

    return idx


def parse_tasm(file, no_warning=False):
    global defined_routines, warnings
    if no_warning:
        warnings = False
    if not os.path.exists(file):
        print(f"Invalid file path: {file}")
        return
    
    raw = ""
    try:
        raw = open(file, "r").read()
    except:
        print(f"Unable to read {file}")
        return
    
    raw = "\n".join(line.split(";")[0].rstrip() for line in raw.splitlines())

    routines = {}
    current_routine = ""

    # get all defined routines
    for line in raw.splitlines():
        if line.endswith(":"):
            defined_routines.append(line[:-1])

    # parse routines and instructions into the routines dictionary
    for index, line in enumerate(raw.splitlines()):
        index += 1
        if line == "":
            continue
        
        ################### routine ###################
        if line.endswith(":"):
            routine = line[:-1]
            
            if routine in list(routines.keys()):
                display_err_msg(line, index, f"Routine {routine} was already defined at line {routines[routine][0]}.")

            current_routine = routine
            routines[routine] = [index]
            continue
        
        ################### instruction ###################
        # check for indent
        if not line.startswith("    "):
            display_err_msg(line, index, "Instructions must be indented by four spaces.")
            continue
        
        # check for routinue marker
        if current_routine == "":
            print(f"{current_routine}")
            display_err_msg(line, index, "Instructions must be under a routine.")
            continue
        
        # instruction arg parser
        instruction_line = line[4:].split(" ")
        instruction, *args = instruction_line

        args = " ".join(args)
        for alias, replacement in ALIASES.items():
            args = args.replace(alias, replacement)

        instruction_function_index = validate_instruction(line, index, instruction, args, current_routine)
        # skip the instruction if it is invalid
        if not instruction_function_index:
            continue

        routines[current_routine].append([instruction, instruction_function_index, args.split(", ")])


    # routines cleanup
    non_empty_routines = {}

    for name, instructions in routines.items():
        if len(instructions) < 2 and not no_warning:
            print(f"WARNING: routine '{name}' on line {instructions[0]} does not have any instructions declared. Ignoring...")
            continue

        non_empty_routines[name] = instructions

    routines = non_empty_routines
    if "_start" not in routines and not no_warning:
        print(f"WARNING: no _start routine found. An automatic start block will not be placed.")


    return routines
    

def determine_groups(routines, display_namespace=False):
    global start_group
    names = {}
    
    rnts = list(routines.items())
    for rnt in rnts:
        name, *triggers = rnt
        group = len(names)
        names[name] = {"group": group, "instructions": triggers[0][1:]}
        if name == "_start":
            start_group = group
    
    if display_namespace:
        print("\nTRIGGERS AND GROUPS OF EACH SRT")
        pretty.pprint(names, expand_all=False)
    return names


def parse_namespace(namespace, group_offset=0, coll_block_offset=0, counter_offset=0, routine_text=False, squish=True, warnings=True):
    routines = list(namespace.keys())
    objs = [""]
    next_free = len(routines)
    
    lengths = {
        routine["group"]: len(routine["instructions"]) for routine in namespace.values()
    }
    
    start_block = gdobj.ioblock(routines.index("_start") + group_offset, 0, "start", override=True) if "_start" in routines else ""
    
    gdobj.coll_block_offset = coll_block_offset
    for routine_index, data in enumerate(namespace.values()):
        group = data["group"]
        routine_instructions = data["instructions"]
        group += group_offset
        routine_str = routines[routine_index]
        # debug routine text
        if routine_text:
            objs.append(gdobj.text_object_str(
                0, group * 30 + 75, 0.5, 0.5, 0, [], 
                f"{group}: {routine_str}", 0
            ))
        index = 0
        for instr in routine_instructions:
            # args to the fn: group, args of command
            command, fn_index, args = instr
            
            line_str = command + ", " + " ".join(args)

            if gdobj.malloc_count > 0 and command == "MALLOC":
                display_err_msg(line_str, "?", "You cannot MALLOC more than once.", routine_text)
                continue
            
            if gdobj.malloc_count < 1 and command in memory_instructions:
                display_err_msg(line_str, "?", "No memory has been initialised.", routine_text)
            
            arg_list = []
            for arg in args:
                if arg in routines: # replace routine names with their corresponding groups
                    arg_list.append(routines.index(arg) + group_offset)
                elif arg == "MEMSIZE":
                    arg_list.append(gdobj.memory_size)
                elif is_item(arg):
                    arg_list.append(arg[0] + str(int(arg[1:]) + counter_offset))
                elif arg != "":
                    arg_list.append(arg)
                    
            # call the argument handler and get the result
            handler = instructions[command]["args"][fn_index - 1][1]
            # arg_list: args supplied in the instruction
            # group: what group the current object is part of
            # lengths: lengths dict, used for optimization in compare triggers
            # index: index of object in this routine's instruction
            # squish: compress object position?
            # nextfree: next group that is not occupied
            reset_extra_objects()
            reset_extra_groups()

            result_str = handler(
                *arg_list, 
                group=group, 
                lengths=lengths,
                index=index, 
                squish=squish, 
                nextfree=next_free + group_offset,
                group_offset=group_offset,
                subroutine_count=len(routines) # used only in malloc
            )
            
            # if result_str == "" and warnings:
                # print(f"WARNING: {command} with {" ".join(args[0])} does not have a builder function and will NOT generate an object.")
            
            if result_str == "\0":
                continue
            
            
            objs.append(result_str)
            # dbg print
            # print(command, *arg_list, group, "->", result_str)
            index += 1 + gdobj.used_extra_objects
            next_free += gdobj.used_extra_groups
    
    # exit if errors
    if errors > 0:
        return ""
    
    # starting objs   
    if group_offset > 100:  # used to teleport to the triggers
        barrier_block2 = f"1,1,2,105,3,{30 * group_offset},155,2,57,99;"
    
    editor_infotext = gdobj.text_object_str(
        195, 45, 0.25, 0.25, 0, [], "go to the editor for details", 0
    )[0:0]

    # trim objects array and compile string
    objs = [obj for obj in objs if obj != ""] + [start_block, editor_infotext]
    
    if group_offset > 100:
        objs.append(barrier_block2)
    if squish and gdobj.timewarp_trigger:
        time_warp = "1,1935,2,-75,3,15,155,1,13,1,36,1,120,5,64,1,67,1;"
        objs.append(time_warp)
    
    obj_str = ";" + "".join(objs)

    # display object count and return string
    print(f"final object count: {len(obj_str.split(";")) - 2}")
    print(f"used groups: {next_free - 1}")
    return obj_str
