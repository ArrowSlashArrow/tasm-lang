from gdobj import *


INSTRUCTIONS = {
    "PERS": {
        "allowed": ["_init"],
        "args": [
            [["item"], make_persistent]
        ]
    },
    "DISPLAY": {
        "allowed": ["_init"],
        "args": [
            [["item"], display_item],
            [["item", "number"], display_item_pos]
        ]
    },
    "INITMEM": {
        "allowed": ["_init"],
        "args": [
            [["int_array"], initmem]
        ]
    },
    "MALLOC": {
        "allowed": ["_init"],
        "args": [
            [["int"], malloc]
        ]
    },
    "MFUNC": {  # read/writes based on mode to register
        "allowed": ["*"],
        "args": [
            [[], mfunc]
        ]
    },
    "MREAD": {  # sets to read mode
        "allowed": ["*"],
        "args": [
            [[], mread]
        ]
    },
    "MWRITE": {  # sets to write mode
        "allowed": ["*"],
        "args": [
            [[], mwrite]
        ]
    },
    "MPTR": {  # moves pointer
        "allowed": ["*"],
        "args": [
            [["int"], mptr]
        ]
    },
    "MRESET": {  # resets pointer to addr 0
        "allowed": ["*"],
        "args": [
            [[], mreset]
        ]
    },
    "IOBLOCK": {
        "allowed": ["_init"],
        "args": [
            [["routine", "int", "str"], ioblock]
        ]
    },
    "NOP": {
        "allowed": "*",
        "args": [
            [[], nop]
        ]
    },
    "MOV": {  # moves arg1 into arg 0 (copy)
        "allowed": "*",
        "args": [
            [["item", "number"], mov_num],
            [["item", "item"], mov_counter],
        ]
    },
    "ADD": { # arg0 += arg1 
        "allowed": "*",
        "args": [
            [["item", "number"], add_num],  # += int
            [["item", "item"], add_counter],  # += counter
            [["item", "item", "item"], add2] # = c1 + c2
        ]
    },
    "SUB": {  # subtracts arg1 from arg0
        "allowed": "*",
        "args": [
            [["item", "number"], sub_num],  # -= int
            [["item", "item"], sub_counter],  # -= counter
            [["item", "item", "item"], sub2] # = c1 - c2
        ]
    },
    "MUL": {  # multiplies arg1 by arg0
        "allowed": "*",
        "args": [
            [["item", "number"], mul_num],  # *= int
            [["item", "item"], mul_counter],  # *= counter
            [["item", "item", "item"], mul2], # = c1 * c2
            [["item", "item", "number"], mul2num] # = c2 * num
        ]
    },
    "DIV": {  # divides arg1 by arg0
        "allowed": "*",
        "args": [
            [["item", "number"], div_num],  # /= int
            [["item", "item"], div_counter],  # /= counter
            [["item", "item", "item"], div2], # = c1 / c2
            [["item", "item", "number"], div2num], # = c1 / num
        ]
    },
    "FLDIV": {  # divides arg1 by arg0
        "allowed": "*",
        "args": [
            [["item", "number"], fldiv_num],  # //= int
            [["item", "item"], fldiv_counter],  # //= counter
            [["item", "item", "item"], fldiv2], # = c1 // c2
            [["item", "item", "number"], fldiv2num], # = c2 // num
        ]
    },
    "SPAWN": {
        "allowed": "*",
        "args": [
            [["routine"], spawn_group]
        ]
    },
    "SE": {
        "allowed": "*",
        "args": [
            [["routine", "item", "number"], spawn_equals_num],
            [["routine", "item", "item"], spawn_equals_item]
        ]
    },
    "SNE": {
        "allowed": "*",
        "args": [
            [["routine", "item", "number"], spawn_nequals_num],
            [["routine", "item", "item"], spawn_nequals_item]
        ]
    },
    "SL": {
        "allowed": "*",
        "args": [
            [["routine", "item", "number"], spawn_less_num],
            [["routine", "item", "item"], spawn_less_item]
        ]
    },
    "SLE": {
        "allowed": "*",
        "args": [
            [["routine", "item", "number"], spawn_lequals_num],
            [["routine", "item", "item"], spawn_lequals_item]
        ]
    },
    "SG": {
        "allowed": "*",
        "args": [
            [["routine", "item", "number"], spawn_greater_num],
            [["routine", "item", "item"], spawn_greater_item]
        ]
    },
    "SGE": {
        "allowed": "*",
        "args": [
            [["routine", "item", "number"], spawn_gequals_num],
            [["routine", "item", "item"], spawn_gequals_item]
        ]
    },
    "FE": {
        "allowed": "*",
        "args": [
            [["routine", "routine", "item", "number"], fork_equals_num],
            [["routine", "routine", "item", "item"], fork_equals_item]
        ]
    },
    "FNE": {
        "allowed": "*",
        "args": [
            [["routine", "routine", "item", "number"], fork_nequals_num],
            [["routine", "routine", "item", "item"], fork_nequals_item]
        ]
    },
    "FL": {
        "allowed": "*",
        "args": [
            [["routine", "routine", "item", "number"], fork_less_num],
            [["routine", "routine", "item", "item"], fork_less_item]
        ]
    },
    "FLE": {
        "allowed": "*",
        "args": [
            [["routine", "routine", "item", "number"], fork_lequals_num],
            [["routine", "routine", "item", "item"], fork_lequals_item]
        ]
    },
    "FG": {
        "allowed": "*",
        "args": [
            [["routine", "routine", "item", "number"], fork_greater_num],
            [["routine", "routine", "item", "item"], fork_greater_item]
        ]
    },
    "FGE": {
        "allowed": "*",
        "args": [
            [["routine", "routine", "item", "number"], fork_gequals_num],
            [["routine", "routine", "item", "item"], fork_gequals_item]
        ]
    },
}