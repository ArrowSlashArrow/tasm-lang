---
label: Home
icon: home
---

# TASM Documentation
# 1. Overview 
## 1.1. Abstract 
TASM is a powerful, domain-specific language that is designed to take advantage of the trigger system in Geometry Dash. The language is intended as an alternative to hand-placement of triggers in a level, and encourages developers to instead write clean code to achieve the same. 
The language is theoretically turing-complete, assuming unbounded IDs, however, GD imposes strictly unavoidable [constraints](#21-constraints). 
A powerful instruction set is provided, which allows for looping, branching, as well as a versatile SDK.

Quick links:
- [Available Instructions](#312-available-instructions)
- [Instruction Flags](#314-instruction-flags)
- [Group Usage](#35-group-usage)
- [Special Routines](#322-special-routines)
- [Example Programs](#441-example-programs)
- [Types of Values](#33-types-of-values)
- [TASM Toolkit](#4-tasm-toolkit)
- [Compiler options](#412-compiler-options)
## 1.2. Terms and definitions 
### 1.2.1. IOBlock 
An IOBlock is a structure that consists of the following:
- A default block
- A multi-activate touchable spawn trigger for the corresponding group (to activate it)
- A text label, preferably one that describes the purpose of the IOBlock (except for the starting IOBlock, which is always labelled `start`)
IOBlocks are intended as a mechanism for the creator to test the functionality of the program in-level by hitting the block with your player.
### 1.2.2. Argset
Abbreviation for "Argument Set". Simply, a set of arguments passed to an instruction.
```x86asm
INSTRUCTION a, b, c
```
Here, `[a, b, c]` is the argset for the instruction.
### 1.2.3. n-tick
n-tick refers to the execution time of any single instruction. A 1-tick instruction takes exactly one tick to execute.  
To be clear, no instructions have a delay of execution. The execution time refers to how long the instruction takes to process.

> [!NOTE]
> All execution times in this document assume that the program is compiled in **release mode** (`--release`). In the default (debug) mode, instructions are spread further apart to make them readable in the editor, which makes every tick of spacing 30 ticks long. See [In-level object representation](#323-in-level-object-representation).
### 1.2.4. Space
A space is the horizontal slot that an instruction occupies in its routine. Most instructions occupy exactly one space, which corresponds to one tick in release mode. Some instructions occupy no space at all (e.g. `RAW`, `BREAKPOINT`), and wait instructions occupy as many spaces as the number of ticks that they wait.

## 1.3. Version and updating

The version is defined according to [semantic versioning](https://semver.org).
### 1.3.1. Current version
<!-- Version number -->
The current version, as of September 9, 2026 is **v0.3.2**. 
Development of the project can be found on the [TASM repo](https://github.com/ArrowSlashArrow/tasm-lang).
# 2. The GD environment
This section contains documentation of the GD environment that is relevant to the purposes and function of TASM and/or the compiler.
## 2.1. Constraints
While TASM is theoretically turing-complete, assuming unbounded IDs, the GD environment imposes strict limits that are impossible to bypass. As a consequence, TASM programmers must be aware of these constraints and their implications.
- The range of IDs normally accessible from triggers is \[1, 10000). This applies to all IDs: groups, counters, timers, collision blocks, etc. It is possible to access further items with remaps, most notably in Item Edit triggers, however this behaviour is not officially supported in GD and may lead to instability. The compiler will refuse to compile a program that uses more than 9,999 groups (error E0001).
- Counter items (counters) are 32-bit signed integers. They may hold any value from \[-2<sup>31</sup>, 2<sup>31</sup>-1].
- Timer items (timers) are 32-bit floats, as per the [IEEE-754](https://en.wikipedia.org/wiki/Single-precision_floating-point_format) implementation. It is not possible to set the value of a timer higher than 9,999,999.0 with item edit triggers. However, time triggers are able to do this just fine.
- The game runs on a 240Hz loop, which means that 1 tick in TASM takes, in theory, exactly 1/240th of a second (~4.166ms).
- This section covers a few constraints of the GD runtime relevant to TASM. In reality, there are hundreds of undocumented edge cases. For known mechanics/bugs, see [this website](https://uhdanke.github.io/gd_docs/).

# 3. The TASM language 
## 3.1. Instructions 
### 3.1.1. Instruction syntax 
Instructions are written by their identifier followed by a space, followed by comma-separated arguments. Optionally, [flags](#314-instruction-flags) may follow after a `|`.
If an instruction takes no arguments, simply the instruction identifier is enough.
Examples:
```x86asm
INSTRUCTION argument1, argument2
INSTRUCTION argument1, argument2 | flag:value
NOARGS
```

A few rules to keep in mind:
- Instruction identifiers are **case-insensitive**: `MOV`, `mov` and `Mov` are all the same instruction. Flag identifiers, on the other hand, are case-sensitive and must be written in lowercase.
- Every instruction line must be indented (by any amount of spaces or tabs) under a routine label.
- Trailing commas are not allowed (error E0022).

It is important to know that instruction arguments (argsets) are typed to ensure differentiation between different functions of an instruction.
For example, the `SE` instruction is used as a branch instruction. It allows both the comparison of an item to a number and two items to each other:
```x86asm
SE example_routine1, C1, 0
SE example_routine2, C1, C2
```
Note that instruction argsets are typed to ensure that valid arguments are passed. Learn more in [this section](#337-argsets).

#### 3.1.1.1. How to read the instruction reference
Each instruction in the next section is documented with some or all of the following:
- **Arguments**: the accepted argsets. Types are written in angle brackets, e.g. `<item>`. See [Types of values](#33-types-of-values) for what each type accepts.
- **Flags**: the [flags](#314-instruction-flags) that the instruction reads. Flags that are passed to an instruction that does not read them are silently ignored, so double-check this list when a flag seems to have no effect.
- **Execution time**: how many ticks the instruction occupies (in release mode).
- **Objects**: what the instruction compiles to in the level.
- **Groups**: extra groups used, in addition to the routine's own group.
- **`_init` only**: present if the instruction may only be used in the [`_init` routine](#3222-_init-routine).

### 3.1.2. Available instructions 
All instructions in this section are expected to be fully functional. Any deprecated instructions will not be listed as of the next minor release.

If an instruction does not have a specified execution time, assume that it is a 1-tick instruction.

Summary of all instructions:

| Category | Instructions |
| -------- | ------------ |
| [Arithmetic](#3121-arithmetic) | `MOV`, `ADD`, `SUB`, `MUL`, `DIV`, `FLDIV`, `ADDM`, `SUBM`, `ADDD`, `SUBD` |
| [Compare](#3122-compare) | `SE`, `SNE`, `SL`, `SLE`, `SG`, `SGE`, `FE`, `FNE`, `FL`, `FLE`, `FG`, `FGE`, `SRAND`, `FRAND`, and all `I`-prefixed instant variants |
| [Collision](#3123-collision-checkers) | `INSTCOLL`, `COLL` |
| [Process](#3124-process) | `SPAWN`, `PAUSE`, `RESUME`, `KILL`, `TOGGLEON`, `TOGGLEOFF`, `EVENT` |
| [Wait](#3125-wait) | `NOP`, `WAIT`, `WAITS` |
| [Time](#3126-time) | `TSPAWN`, `TSTART`, `TSTOP` |
| [Miscellaneous](#3127-miscellaneous) | `PERS`, `UNPERS`, `RPERSALL`, `UNPERSALL`, `DISPLAY`, `IOBLOCK`, `BREAKPOINT` |
| Compiler directives | [`ALIAS`](#3128-the-alias-instruction), [`RAW`](#3129-the-raw-instruction), [`RAWTRG`](#31291-the-rawtrg-instruction), [`IMPORT`](#31210-the-import-instruction), [`IMPORTSTD`](#31211-importing-from-the-standard-library-importstd) |

#### 3.1.2.1. Arithmetic
All arithmetic instructions are 1-tick, compile to a single Item Edit trigger, and use no extra groups.  
By convention, the counter that stores the result of an arithmetic operation is usually specified as the first argument.
##### Argument format

| Argset                   | Result (in the example case of division) | Commands that use it           |
| ------------------------ | ---------------------------------------- | ------------------------------ |
| `<item> <number>`        | `item = item / number`                   | ADD, SUB, MUL, DIV, FLDIV, MOV\* |
| `<item> <item>`          | `1st item = 1st item / 2nd item`         | ADD, SUB, MUL, DIV, FLDIV, MOV\* |
| `<item> <item> <number>` | `1st item = 2nd item / number`           | MUL, DIV\*\*, FLDIV\*\*          |
| `<item> <item> <item>`   | `1st item = 2nd item / 3rd item`         | ADD, SUB, MUL, DIV, FLDIV      |

Note: Neither `MAINTIME` nor `ATTEMPTS` can ever be the result. Those items are immutable, and the compiler emits error E0004 if they are written to. `POINTS` may be written to.
##### Instruction operations
- ADD: addition
- SUB: subtraction
- MUL: multiplication
- DIV: division
- FLDIV: division, and the result is rounded down (floored).
- MOV: assignment
> \* MOV simply assigns the 2nd argument to the 1st item in this case. Data is not transformed when MOV is used. MOV only accepts the first two argsets.

> [!WARNING]
> \*\* In the current compiler (v0.3.2), the `<item> <item> <number>` form of `DIV` and `FLDIV` compiles with the modifier set to *multiply*, i.e. `DIV C1, C2, 4` produces `C1 = C2 * 4`. Until this is fixed, add `| divmod:true` to get a division: `DIV C1, C2, 4 | divmod:true` produces `C1 = C2 / 4`.

##### Flags
All arithmetic instructions read the following flags (see [Instruction flags](#314-instruction-flags) for details):

| Flag | Effect | Default |
| ---- | ------ | ------- |
| `itemmod` | Overrides the modifier. In the `<item> <number>` and `... <number>` forms, the modifier *is* the number argument, so this flag replaces it. | The number argument, or `1` |
| `divmod` | `true` divides the item result by the modifier instead of multiplying. | `false` |
| `op` | Operator between the two input items. Only meaningful in the `<item> <item> <item>` form. | The instruction's operator |
| `iter` | Assignment operator to the target item (`+` behaves like `+=`). | The instruction's operator for the 1- and 2-operand forms; `=` for the `<item> <item> <number>` and `<item> <item> <item>` forms |
| `resmode` | Rounding/sign of the item result. | none (`floor` for the 3-argument forms of `FLDIV`) |
| `finmode` | Rounding/sign of the final result. Only applied when the assignment operator is not `=`. | none (`floor` for the 2-argument forms of `FLDIV`) |

Example:
```x86asm
abs_value:
    MOV C1, C1 | resmode:+        ; C1 = |C1|
    ADD C2, C3 | itemmod:0.5      ; C2 = C2 + C3 * 0.5
```

##### 3.1.2.1.2. Add/Subtract with modifier
These instructions are utility instructions included to shorten common expressions.
###### `ADDM`
Arguments:
- `ADDM <item> <item> <number>`: Adds the 2nd item multiplied by the number to the 1st item.
- `ADDM <item> <item> <item> <number>`: Adds the 2nd item and 3rd item together, multiplies their sum by the number, and adds the product to the 1st item.

This instruction performs addition and multiplication within the same tick. Useful for shortening expressions which follow the form of `result = result + operand * modifier`.
###### `SUBM`
Arguments:
- `SUBM <item> <item> <number>`: Subtracts the 2nd item multiplied by the number from the 1st item.
- `SUBM <item> <item> <item> <number>`: Subtracts the 3rd item from the 2nd item, multiplies their difference by the number, which is then subtracted from the 1st item.

This instruction performs subtraction and multiplication within the same tick. Useful for shortening expressions which follow the form of `result = result - (operand * modifier)`.
###### `ADDD`/`SUBD`
These instruction accept the same arguments and do the same thing as their `ADDM`/`SUBM` counterpart, except that the result between the operand and number is division instead of multiplication.  
Useful for shortening expression which follow the form of:
- for `ADDD`, `result = result + operand / modifier`
- for `SUBD`, `result = result - (operand / modifier)`

Flags: `itemmod`, `divmod`, `op`, `iter` for all forms; `resmode` and `finmode` for the 3-argument forms only (the 4-argument forms ignore them).

> [!WARNING]
> In the current compiler (v0.3.2), the multiply/divide modifier of these instructions is swapped: `ADDM`/`SUBM` compile with a *dividing* modifier and `ADDD`/`SUBD` compile with a *multiplying* modifier. Until this is fixed, the intended behaviour can be forced with the `divmod` flag:
> - `ADDM`/`SUBM`: append `| divmod:false` to multiply.
> - `ADDD`/`SUBD`: append `| divmod:true` to divide.

#### 3.1.2.2. Compare
Spawning a group does not automatically pause the parent group.  
All compare instructions are 1-tick.  

Execution timeline (base compares):
- Tick n: the compare trigger is called
- Tick n + 1: 
	- the compare trigger calls the intermediate group spawner trigger if it should be called
	- the parent group executes the next instruction.
- Tick n + 2: the spawned group starts execution

Execution timeline (instant compares):
- Tick n: the compare trigger is called
- Tick n + 1: the spawned group starts execution

##### Compare flags
All compare instructions (base and instant) can modify each side of the comparison before it is evaluated. Each side is evaluated as `(value <modop> mod)`, then rounded/signed according to its mode:

| Flag | Effect | Default |
| ---- | ------ | ------- |
| `lmod` / `rmod` | Modifier of the left/right-hand side. | `1` for the left side. For the right side: the number argument if one is given, otherwise `1`. |
| `lmodop` / `rmodop` | Operator between a side and its modifier (`+`, `-`, `*`, `/`). | `*` |
| `lmode` / `rmode` | Rounding/sign mode of a side. | none |

> [!NOTE]
> When the right-hand side is a number literal, GD stores that number as the right side's modifier. Passing `rmod` therefore **replaces** the number: `SE g5, C1, 10 | rmod:2` compares `C1` against `2`.

Base (non-instant) compares additionally read the flags for their intermediate spawn triggers:

| Flag | Applies to | Effect | Default |
| ---- | ---------- | ------ | ------- |
| `ordered` | Spawn and fork compares, `SRAND`, `FRAND` | Whether the spawned routine(s) are spawned with spawn ordered. | `true` |
| `noremap` | Spawn compares, `SRAND` | Enables `reset remap` on the spawn trigger. | `false` |
| `lnoremap` | Fork compares, `FRAND` | Enables `reset remap` on the spawn trigger of the first (truthy) routine. | `false` |
| `rnoremap` | Fork compares, `FRAND` | Enables `reset remap` on the spawn trigger of the second (falsy) routine. | `false` |

Instant compares do not have intermediate spawn triggers, so they ignore `ordered`, `noremap`, `lnoremap` and `rnoremap`.
##### Spawn if
`SE`, `SNE`, `SL`, `SLE`, `SG`, `SGE` all accept: `<routine> <item> <number>`, `<routine> <item> <item>`

The specified routine is spawned if the two arguments meet the condition.
* SE: Spawns routine if a == b
* SNE: Spawns routine if a != b
* SL: Spawns routine if a < b
* SLE: Spawns routine if a <= b
* SG: Spawns routine if a > b
* SGE: Spawns routine if a >= b

> Here, a and b refer to the first and second operands respectively.

Flags: `lmod`, `rmod`, `lmodop`, `rmodop`, `lmode`, `rmode`, `ordered`, `noremap`.  
Objects: 1 Item Compare trigger and 1 spawn trigger.  
Groups: 1.  
Execution time: 1 tick.

Example:
```x86asm
loop:
    ADD C1, 1
    SL loop, C1, 10                     ; repeat while C1 < 10
    SE done, C1, C2 | lmode:+ ordered:false  ; spawn `done` if |C1| == C2, unordered
```

##### Spawn if-else (Fork)
`FE`, `FNE`, `FL`, `FLE`, `FG`, `FGE` all accept: `<routine> <routine> <item> <number>`, `<routine> <routine> <item> <item>`

The first routine is spawned if the two arguments meet the condition. Otherwise, the second routine is spawned.
* FE: Spawns first routine if a == b otherwise spawns the second routine.
* FNE: Spawns first routine if a != b otherwise spawns the second routine.
* FL: Spawns first routine if a < b otherwise spawns the second routine.
* FLE: Spawns first routine if a <= b otherwise spawns the second routine.
* FG: Spawns first routine if a > b otherwise spawns the second routine.
* FGE: Spawns first routine if a >= b otherwise spawns the second routine.

> Here, a and b refer to the first and second operands respectively.

Flags: `lmod`, `rmod`, `lmodop`, `rmodop`, `lmode`, `rmode`, `ordered`, `lnoremap`, `rnoremap`.  
Objects: 1 Item Compare trigger and 2 spawn triggers.  
Groups: 2.  
Execution time: 1 tick.

##### Instant compares
"Instant" compares can be accessed by prefixing a compare instruction with `I`. These instructions directly spawn the target groups instead of using any intermediate triggers.
This means that spawned groups will not be spawned with spawn ordered or will be able to be paused/stopped. This feature is intended for fast execution of routines. Note that remaps are still carried through any instant compares.

All available instant compare instructions:
- ISE, ISNE, ISL, ISLE, ISG, ISGE
- IFE, IFNE, IFL, IFLE, IFG, IFGE
- ISRAND, IFRAND

All instant compare instructions take the same arguments as their base counterparts.  
Flags: `lmod`, `rmod`, `lmodop`, `rmodop`, `lmode`, `rmode` (not applicable to `ISRAND`/`IFRAND`, which read no flags).  
Objects: 1 trigger (Item Compare or Random).  
Groups: 0.  
Execution time: 1 tick.

##### Random Spawns
`SRAND <routine> <number>`

Spawns the first routine according to the supplied chance (2nd argument).  
The second argument (the chance) should be a float in the range \[0.0, 100.0].

`FRAND <routine> <routine> <number>`

Spawns the first routine according to the supplied chance (2nd argument).
If the first routine is not spawned, the second routine is spawned.  
The third argument (the chance) should be a float in the range \[0.0, 100.0].

Examples:
```x86asm
SRAND do_stuff, 42.8
```
The routine `do_stuff` has a 42.8% chance of being spawned.

Flags:
- `SRAND`: `ordered`, `noremap`.
- `FRAND`: `ordered`, `lnoremap`, `rnoremap`.

Objects: 1 Random trigger plus 1 (`SRAND`) or 2 (`FRAND`) spawn triggers.  
Groups: 1 (`SRAND`) or 2 (`FRAND`).  
Execution time: 1 tick. Like the base compares, the target routine starts executing 2 ticks after the instruction is called.

#### 3.1.2.3. Collision checkers
These instructions check collisions between two hitboxes. They primarily use the collision and instant collision triggers.
##### INSTCOLL
Arguments: `INSTCOLL <group> <group> <hitbox> <hitbox>`
Spawns the first group if the hitboxes are colliding at the instant that this instruction runs; spawns the second group if that is not the case. This instruction compiles to an instant collision trigger.

Both groups are required. A group cannot be `0` (`0` is a number, not a group, and `g0` is not a valid group ID), so if one of the outcomes should do nothing, point it at an empty routine.

Unlike compare instructions, this instruction *does not* use intermediate spawn triggers to ensure spawn order. Consider spawning an intermediate routine for each routine to make them spawn-ordered or stoppable/killable. 

See [this section](#3341-valid-hitbox-combinations) for valid combinations of the third and fourth arguments.  
Flags: none.  
Execution time: 1 tick.
##### COLL
Arguments: `COLL <group> <hitbox> <hitbox>`
Spawns the target group when the two hitboxes collide. This trigger will spawn the group whenever the two hitboxes collide, for as long as it is active.

See [this section](#3341-valid-hitbox-combinations) for valid combinations of the second and third arguments.  
Flags: none usable (see note).  
Execution time: 1 tick.

> [!NOTE]
> The handler for `COLL` is written to read the `nospawn` (toggle the target group off instead of spawning it) and `onexit` (trigger when the hitboxes stop colliding) flags, but as of v0.3.2 these flags are not registered with the flag parser, so using them results in error E0031 (`Unrecognized flag`).
#### 3.1.2.4. Process
These instructions are responsible for managing running groups. This includes spawning them, stopping and killing them.
> Note that the terms "routine" and "group" refer to essentialy the same thing in this section. The distinction between the two terms stems from the knowledge of the contents on the group; where a routine has known triggers (as is specified in a program), and a group may have some arbitrary external objects. All routines are treated as groups in GD.
##### SPAWN
Arguments: `SPAWN <routine>`

Spawns the corresponding routine. Does not pause the current group.  
By default, the generated spawn trigger has `spawn ordered` set to `true`. This can be disabled with `ordered:false`.

Flags:

| Flag | Effect | Default |
| ---- | ------ | ------- |
| `delay` | Spawn delay in seconds. | `0.0044` (~1 tick) |
| `ordered` | Whether the routine is spawned with spawn ordered. | `true` |
| `remap` | ID remap dictionary: `{old=new, ...}`. | none |
| `noremap` | Enables `reset remap` on the spawn trigger. | `false` |

Example:
```x86asm
_start:
    SPAWN worker | delay:0.5 ordered:false
    SPAWN mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
```

Execution time: 1 tick.  

##### PAUSE
Arguments: `PAUSE <routine>`

Pauses the specified routine via a stop trigger.
> A routine cannot be paused immediately after is it spawned. If it paused this way, the pause will simply be overshadowed by GD. A minimum of one tick is needed between the execution of the first non-wait instruction of the routine and the PAUSE for it to function. 

Flags: none.  
Execution time: 1 tick.

##### RESUME
Arguments: `RESUME <routine>`

Resumes execution of a paused routine via a stop trigger.  
Flags: none.  
Execution time: 1 tick.

##### KILL
Arguments: `KILL <routine>`

Kills the specified routine via a stop trigger. Unlike `PAUSE`, once a routine is stopped with this instruction, it is **irreversible** and cannot be resumed.  
Flags: none.  
Execution time: 1 tick.

##### TOGGLEON
Arguments: `TOGGLEON <group>`

Toggles the argument group on via toggle trigger.  
Flags: none.  
Execution time: 1 tick.

##### TOGGLEOFF
Arguments: `TOGGLEOFF <group>`

Toggles the argument group off via toggle trigger.  
Flags: none.  
Execution time: 1 tick.

##### EVENT
Arguments: `EVENT <group> <int>`

Spawns the target group when the event ID happens. This trigger will always listen for events once started and cannot be turned off.
The `<int>` argument must be an integer that corresponds to one of the event IDs in GD (see the [list of event IDs](https://docs.rs/gdlib/latest/gdlib/cclocallevels/gdobj/structs/enum.Event.html)). An unknown ID results in error E0019.

Flags:
- `extra1`: the "Extra ID" option of the event trigger (float, truncated to an integer).
- `extra2`: the "Extra ID 2" option of the event trigger (float, truncated to an integer). See the [list of accepted values](https://docs.rs/gdlib/latest/gdlib/cclocallevels/gdobj/structs/enum.ExtraID2.html).

`_init` only. Like any other trigger in `_init`, it is placed to the left of x=0 and becomes active when the level starts.
#### 3.1.2.5. Wait
> [!NOTE]
> These instructions wait only if the routine that they are in is spawned with spawn ordered. They do nothing otherwise.

Wait instructions do not compile to any objects; they leave empty space in the routine instead. They read no flags.
##### NOP
Arguments: `NOP`

Does nothing on the tick it is called (equivalent to `WAIT 1`). Useful for waiting.  
Execution time: 1 tick.  
##### WAIT
Arguments: `WAIT <int>`

Does nothing for the following n ticks. Effectively a series of `NOP`s.  
Wait time cannot be negative. The compiler throws an error (E0006) if it is specified as such. 

Execution time: n ticks.
##### WAITS
Arguments: `WAITS <number>`

Waits the specified number of seconds, where the amount of time is converted to ticks using `floor(seconds * 240)`. Effectively the same as `WAIT`.  
Unlike `NOP` and `WAIT`, `WAITS` waits the same real-world time in both release and debug mode (in debug mode, it is converted to `floor(seconds * 8)` spaces of 30 ticks each).  
Execution time: variable.
#### 3.1.2.6. Time
Timer arguments must be timer literals (e.g. `T5`). Built-in items such as `MAINTIME` are not accepted.
##### TSPAWN
Arguments: `TSPAWN <timer> <number> <number> <routine>`

Starts the timer at the given time (2nd argument), and spawns the given routine when the timer reaches the target time (3rd argument).  
The timer can be written to during the period that it is ticking. This is not recommended for usage of timer items as timers.

Flags:

| Flag | Effect | Default |
| ---- | ------ | ------- |
| `tpaused` | Starts the target timer paused. | `false` |
| `tstop` | Stops the target timer once the target time has been reached. | `false` |
| `tmod` | Time multiplier for the timer. Can be negative. | `1.0` |
| `nover` | "Don't override": only activate if the target timer is not running, or it is at 0.00, or `tpaused` is enabled. | `false` |

Example:
```x86asm
_start:
    TSPAWN T1, 0, 5, on_timeout | tstop:true   ; spawn on_timeout after 5 seconds, then stop T1
```

Execution time: 1 tick.
##### TSTART
Arguments: `TSTART <timer>`

Unpauses the specified timer. To be started, the timer must have first been started by a time trigger via `TSPAWN`.  
Flags: none.  
Execution time: 1 tick.

##### TSTOP
Arguments: `TSTOP <timer>`

Pauses a running timer.  
Flags: none.  
Execution time: 1 tick.
#### 3.1.2.7. Miscellaneous
All instructions in this section provide functionality that does not neatly fit into any other category. 
##### 3.1.2.7.1. Persistent item trigger
All instructions in this section have an execution time of 1 tick, compile to a single Persistent Item trigger, and read no flags. They may be used in any routine, including `_init`.
###### PERS
Arguments: `PERS <item>`

Makes the given item persistent.  
###### UNPERS
Arguments: `UNPERS <item>`

Removes the given item's persistence.  
###### RPERSALL
Arguments: `RPERSALL`

Resets all currently persistent items to 0.  
###### UNPERSALL
Arguments: `UNPERSALL`

Removes the persistence of all currently persistent items.

##### DISPLAY
Arguments: `DISPLAY <item>`

Adds a counter object for the corresponding item. Counter displays are stacked upwards at x=-75, starting at y=75, in the order that they are declared.  
Flags: none.  
`_init` only.
##### IOBLOCK
Arguments: `IOBLOCK <group> <int> <string>`

Places a block at the bottom of the level, at the specified x-position (2nd argument) with an annotation (3rd argument).
Also places a touchable spawn trigger that spawns the specified group.
Intended as a debug feature and/or substitute for user input.  

Position `0` is used by the automatically placed `start` IOBlock, so use positions of `1` or greater to avoid overlapping it.  
The label cannot contain commas, `;` or `|`. If it looks like another value (e.g. `g12` or `5`), prefix it with `\`. A label that is exactly the name of a routine is always resolved as that routine, even when escaped, which will fail to compile.

Flags: `ordered` (whether the touch spawn trigger uses spawn ordered; default `true`).  
`_init` only.

Example:
```x86asm
_init:
    IOBLOCK reset, 2, reset counters
    IOBLOCK step, 3, step | ordered:false
```
##### BREAKPOINT
Arguments: `BREAKPOINT`

A debug marker. It does not compile to any objects and takes no space; it is ignored when compiling to a level.  
Execution time: 0 ticks.
#### 3.1.2.8. The `ALIAS` instruction
`ALIAS` is a special instruction that may only be used in the `_init` routine (error E0023 otherwise). It is used for defining custom aliases for values.  
For readability, the following can be rewritten, from: 
```x86asm
_start:
	MOV C1, 10
	MOV C2, 10
	MOV C3, 10
```
to:
```x86asm
_init:
	ALIAS start_value, 10

_start:
	MOV C1, start_value	; becomes MOV C1, 10
	MOV C2, start_value ; and so on...
	MOV C3, start_value
```

This instruction is parsed before any other instruction to build an alias reference table. This is done to be able to resolve the values of referenced aliases when determining types of argsets.  
Aliases are intended for developers to improve the readability of code that uses common values and reduce the usage of magic values.
Defined aliases are global and constant and cannot be re-assigned (error E0013).  
The instruction takes two arguments as input: `ALIAS <string> <value>`.
* The string is the identifier of the alias and the identifier by which it must be referred to. Since aliases are parsed before anything else, aliases are resolved regardless of the location of their definiton within the `_init` routine. The identifier must not look like any other type of value: names such as `C5`, `g2`, `42` or `POINTS` are rejected (error E0015).
* The value may be of any type. This value is what is inserted when an alias is referenced by its identifier.

```x86asm
_init:
	ALIAS value, 42

_start:
	MOV C1, value
	; becomes:
	MOV C1, 42
```

Aliases will **not** clone values from other aliases:
```x86asm
_init:
	ALIAS value, 42		; alias `value` holds 42
	ALIAS value2, value	; alias `value2` holds "value", NOT 42. 
```

Where aliases can be used:
- As a whole instruction argument (an alias is substituted only if it makes up the entire argument).
- As a key or value inside a [Dict flag](#dict), such as `remap`.
- From another module with `module::alias` syntax (see [Using external symbols](#341-using-external-symbols)).

Aliases are **not** substituted inside non-Dict flag values: `ADD C1, C2 | itemmod:half` does not compile, even if `half` is an alias.

#### 3.1.2.9. The `RAW` instruction
> [!NOTE]
> This instruction is a feature intended for advanced users. 

The `RAW` instruction inserts the given object string directly into the resulting level. Since TASM does not have dedicated instructions for each individual trigger, it is necessary for this instruction to exist to allow for the insertion of arbitrary objects.  
This instruction expects only one argument: `RAW <objects>`. Everything after the instruction identifier is taken as-is (it is not split on commas), and must strictly be a **raw** object string, which is *NOT* the same thing as a .gmd file.  
The instructions inserts the object string according to the GDLib's GDObject constructor, which prevents the creation of degenerate object with missing properties that may cause the level not to load. This may lead to strange formations in the level in the case of a malformed input.  
One may obtain an object string by using the BetterEdit mod for Geode, and simply pressing ctrl+c to copy the object(s).

> [!IMPORTANT]
> Since `;` starts a comment in TASM, and objects in an object string are separated by `;`, only the first object of a copied object string is kept. Insert one object per `RAW` instruction, and leave out the trailing `;`. The object string may also not contain a `|`, since that character starts the flag section.

```x86asm
_init:
    RAW 1,1,2,15,3,15     ; places a block (object ID 1) at (15, 15)
```

> [!WARNING]
> The objects that it controls are not expected to be related to the routine in which this instruction was written. Since a raw object string is included, the objects may be of any abitrary group(s), at any arbitrary positions, and be otherwise totally unrelated to the routine.  
> Please double-check and comment usages of this instruction thoroughly. Object strings are notoriously opaque and difficult to read, which makes them very prone to accidental misformatting.

Flags: none.  
Execution time: 0 ticks (it takes no space in the routine).
##### 3.1.2.9.1. The `RAWTRG` instruction
Much like the `RAW` instruction, the `RAWTRG` instruction takes an object string and inserts it into the level, except that all given objects are treated as part of the routine. This has the follow implications: 
- All given objects are placed at the compiler-assigned position in the respective routine, in the respective instruction position. 
- All given objects are appended to the group of routine
- All given objects are made spawn-triggerable and multi-triggerable.

This instruction is to give the programmer the option to place a trigger that is not yet supported by TASM in the usual position and have the compiler treat that object as a trigger in the routine.  
The same object string restrictions as `RAW` apply.

Flags: none.  
Execution time: 1 tick.

#### 3.1.2.10. The `IMPORT` instruction
The `IMPORT` instruction is a way for the programmer to include library files to use their code in the program. This instruction expects only one argument - a path to the library file. This path must be relative to this file's location but without the `.tasm` extension. 

The module is referred to by its file name (without the extension), regardless of the directories in its path. For example, after `IMPORT libs/math`, the module's symbols are referenced as `math::symbol`.

This instruction is exclusive to the `_init` routine (error E0034 otherwise).

##### Usage Example
For this example, this directory structure is assumed:
```x86asm
project/
	main.tasm
	library.tasm
```
For `main.tasm` to use the library, the `IMPORT` instruction must declare it:
```x86asm
_init:
	; this instruction imports `library.tasm`
	IMPORT library

; now, we can use symbols from the library.
```

For more documentation on the module system, refer to [this section](#34-external-symbols-and-the-module-system).

#### 3.1.2.11. Importing from the standard library (`IMPORTSTD`)
The IMPORTSTD instruction behaves identically to the IMPORT instruction, except that the given path is resolved relative to the `stdlib` directory of the standard TASM installation, rather than relative to the current file's location. This instruction expects only one argument: a path to the library file, relative to the installation's stdlib directory and without the .tasm extension.

To ensure that this instruction works properly, please install TASM via the provided install scripts (see [Installing TASM](#41-installing-tasm)). The installation directory is:
- Windows: `%APPDATA%\tasm`
- Linux: `~/.local/bin/tasm`

> [!WARNING]
> - The standard library is only bundled with releases newer than v0.3.2. With v0.3.2 and older, copy the `stdlib/` folder from the repository yourself, or use `IMPORT` with a relative path instead.
> - In the v0.3.2 compiler, the lookup path contains `stdlib` twice, i.e. `importstd math` is looked up at `<install dir>/stdlib/stdlib/math.tasm`. If a standard import cannot be found (error E0043), check that location.

Like IMPORT, this instruction is exclusive to the _init routine.

##### 3.1.2.11.1. Example standard import
Assuming a standard TASM installation with the following stdlib structure:

```
<tasm install dir>/
	stdlib/
		math.tasm
		collections/
			list.tasm
```

A program can pull in symbols from math.tasm and collections/list.tasm as follows:

```x86asm
_init:
	; imports stdlib/math.tasm
	importstd math

	; imports stdlib/collections/list.tasm
	importstd collections/list

; now, we can use symbols from both library files: math::<symbol> and list::<symbol>
; ...
```
For more documentation on the module system, refer to [this section](#34-external-symbols-and-the-module-system).

#### 3.1.2.12. Excluded instructions (not part of the ISA)
Some instructions were left out in the design process of the ISA that arguably could be very useful, like the `MOD` instruction. Initially the `MOD` instruction was intended as a supplement to the arithmetic set of instructions as a utility. However, this instruction was eventually excluded for the instruction set due to consisting of existing instructions. As seen in the [prime number check example](#prime-checker), a modulus is necessary to compute to determine whether a number is factorable by some other number.  
It is clear in that example that the MOD instruction is just a constituent of other arithmetic operations, which is why it was excluded. The primary goal of TASM is to be a direct representation of GD triggers as code. Since there is no trigger that computes the modulus of a number, this operation is excluded.  
Likewise, all bitwise instructions were left out of the TASM instruction set because there are no built-in operations to compute, for instance, a & b.


### 3.1.3. In-level object representation 
All arithmetic instructions use a single Item Edit trigger, including MOV.  
All spawn compare instructions use 2 triggers: one for the Item Compare, to perform the comparison, and one for the group spawner.  
All fork compare instructions use 3 triggers: one for the Item Compare, to perform the comparison, and two for each group spawner.  
`SRAND` uses a Random trigger and one spawn trigger; `FRAND` uses a Random trigger and two spawn triggers.  
All instant compare instructions (including `ISRAND`/`IFRAND`) use a single trigger.

NOP, WAIT and WAITS do not compile to any objects, instead, a blank space is left which acts as a wait since the group will be spawn-ordered.  
SPAWN simply adds a spawn trigger (with spawn-ordered enabled) to the specified group.   
> It should be noted that all group are spawned by a spawn trigger with **spawn-ordered enabled by default**. 

#### 3.1.3.1. Initializer instructions
All initializer instructions correspond to custom in-level structures, which may not necessarily be single triggers. For this reason, they are allowed only as setup instructions.
Below is a list of instructions and their corresponding structures:
- `IOBLOCK`: An [IOBlock](#121-ioblock) that is put at y=75 and some specified x-position that acts as a debug group spawn. The x-position is processed such that it translates to a block position, e.g. 5 becomes 5 blocks (+ 2 for margin) to the right of the y-axis, centered on a cell.
- `DISPLAY`: Displays a counter for the given item at x=-75. Each additional `DISPLAY` is placed one block (30 units) above the previous one, starting at y=75.
- `EVENT`: An event trigger. Unlike the other two, this is a regular trigger and is placed with the other `_init` triggers.
- `ALIAS`: See [this documentation](#3128-the-alias-instruction)
- `IMPORT`: See [this documentation](#31210-the-import-instruction)
- `IMPORTSTD`: See [this documentation](#31211-importing-from-the-standard-library-importstd)

`PERS`, `UNPERS`, `RPERSALL` and `UNPERSALL` are commonly placed in `_init`, but they are regular instructions and may be used in any routine.
### 3.1.4. Instruction flags
Instructions flags extend the functionality of certain instructions.  
The function of a given instruction is usually simple/single-purposed, and only uses a handful of parameters within the trigger that it compiles to. However, triggers are remarkably configurable, and in some cases may simplify otherwise needlessly complex setups.  
A common example is the implementation of getting the absolute value of a number. The old implementation required a comparison of the target (C1) against 0 to determine its sign, which determined whether it should be negated. This is much more complex and wasteful of groups than simply using the absolute rounding mode.
  
Old method:
```
to_positive:
    MUL C1, -1
  
absolute:
    ; check sign
    ; uses two groups due to being a compare instruction
    SL to_positive, C1, 0
```
  
New method, with a flag parameter:
```
absolute:
    MOV C1, C1 | resmode:+
```
The new method uses only one object in total.  

Flags are intended as supplemental customization options to existing instructions for the purpose of fine-grained adjustments to reduce object and group usage. 
They are applied with higher priority than the instruction arguments. For example:
```
routine:
	; itemmod is set to 0.4, which overrides 3.0, even if it specified.
	ADDM C1, C2, 3.0 | itemmod:0.4 
```
#### 3.1.4.1. Supported flags
Flags are written as `flag:value`. The TASM flag parser is very particular, so below are a set of guidelines to follow when passing flags:
- Flags MUST be written after a `|` in the instruction line. There must only be one pipe character in the line if flags are used (error E0021 otherwise).
	- `ADD C1, C2 | itemmod:0.5 resmode:+` compiles.
	- `ADD C1, C2 | itemmod:0.5 | resmode:+` does not compile. 
- Flags must be separated by whitespace. The flag identifier and its value must be separated by a `:`, with no whitespace in between.
	- `... | itemmod:0.5` compiles.
	- `... | itemmod: 0.5` does not compile.
	- `... | itemmod:0.5, resmode:+` does not compile.
- Flag identifiers are case-sensitive and are always lowercase. `ITEMMOD:2` does not compile (error E0031).
- Passing a flag to an instruction that does not read it is not an error; the flag is simply ignored. Refer to the **Flags** line of each instruction to see which flags it reads.
- The above only applies to flags that accept a data type other than Dict. Dicts follow these rules:
	- A Dict is enclosed in braces and contains `key=value` pairs separated by commas: `remap:{1=2, 3=4}`.
	- Whitespace inside the braces and around the `=` is allowed.
	- If any key or value is an imported symbol (`module::symbol`), there **must** be whitespace between the colon and the opening brace: `remap: {module::a=5}`. Otherwise the extra colons are mistaken for the flag separator (error E0030). Writing a space after the colon is therefore recommended for all Dicts.
	- Keys and values may be 16-bit signed integers (decimal, or hexadecimal with `0x`), counter/timer/group/collision block literals (coerced to their ID, e.g. `g5` becomes `5`), routine identifiers (coerced to their group ID), local aliases whose value is one of the above, or imported routines/aliases written as `module::identifier`.

> [!NOTE]
> The types of values that flags accept are different to those listed in the [Types of Values](#33-types-of-values) section. Please refer to the [Flag types](#3142-flag-types) section for more info on accepted values for flags.

> [!NOTE]
> "Item result" refers to the intermediate result between the operands in an item edit trigger (used in arithmetic instructions) that is processed before any additional operations, such as usage of the multiplier or assignment to the target item.
> ![Item Result](item_result.png)

| Flag     | Usage                                                                                                 | Instructions | Type       |
| -------- | ----------------------------------------------------------------------------------------------------- | ------------ | ---------- |
| resmode  | Rounding and sign config for the item result. This is invoked before `finmode`.                       | Arithmetic   | Round/Sign |
| finmode  | Round and sign config for final computed result. Only applied when the assignment operator is not `=`. | Arithmetic   | Round/Sign |
| itemmod  | Modifier in arithmetic instructions. Item result is multiplied by it by default.                      | Arithmetic   | Float      |
| divmod   | Divides item result by modifier rather than multiplying it.                                           | Arithmetic   | Boolean    |
| iter     | Compound assignment operator to target item. In the case of `+`, it functions like `+=`.              | Arithmetic   | Operator   |
| op       | Arithmetic operator between items. Does nothing if there are less than 2 input operands.              | Arithmetic   | Operator   |
| lmodop   | Operator between the left-hand side value and its associated modifier.                                | Compares     | Operator   |
| rmodop   | Operator between the right-hand side value and its associated modifier.                               | Compares     | Operator   |
| lmod     | Modifier of the left-hand side value.                                                                 | Compares     | Float      |
| rmod     | Modifier of the right-hand side value. Replaces the number if the right-hand side is a number literal. | Compares     | Float      |
| lmode    | Rounding and signing mode of the final left-hand side value.                                          | Compares     | Round/Sign |
| rmode    | Rounding and signing mode of the final right-hand side value.                                         | Compares     | Round/Sign |
| delay    | Spawn delay in seconds. Default `0.0044`.                                                             | `SPAWN`      | Float      |
| remap    | ID remap descriptor. Each key-value pair represents the old ID and the new ID respectively.           | `SPAWN`      | Dict       |
| ordered  | Use spawn ordered if true, don't use spawn ordered if false. On by default.                           | `SPAWN`      | Boolean    |
| ordered  | Like the `ordered` flag for `SPAWN`, but only for non-instant compares (including `SRAND`/`FRAND`). Applies to all supplementary spawn triggers. | Compares | Boolean |
| ordered  | Functions exactly like `ordered` for spawn; affects the spawn trigger in the IOBlock.                 | `IOBLOCK`    | Boolean    |
| noremap  | Enables `reset remap` option in the trigger if true.                                                  | `SPAWN`      | Boolean    |
| noremap  | Like the `noremap` flag for `SPAWN`, but only for non-instant spawn compares (including `SRAND`).     | Spawn Compares | Boolean  |
| lnoremap | Like the `noremap` flag for compares, but only applies to the first (truthy) branch's spawn trigger.  | Fork Compares, `FRAND` | Boolean |
| rnoremap | Like the `noremap` flag for compares, but only applies to the second (falsy) branch's spawn trigger.  | Fork Compares, `FRAND` | Boolean |
| tpaused  | Starts target timer paused.                                                                           | `TSPAWN`     | Boolean    |
| tstop    | Stops target timer once the target time has been reached.                                             | `TSPAWN`     | Boolean    |
| tmod     | Time multiplier for timer. Can be negative.                                                           | `TSPAWN`     | Float      |
| nover    | Only activate if the target timer is not running, or it is at 0.00, or the `tpaused` flag is enabled. | `TSPAWN`     | Boolean    |
| extra1   | "Extra ID" option in event triggers.                                                                  | `EVENT`      | Float      |
| extra2   | "Extra ID 2" option in event triggers. See https://docs.rs/gdlib/latest/gdlib/cclocallevels/gdobj/structs/enum.ExtraID2.html | `EVENT` | Float |

> [!NOTE]
> `nospawn` and `onexit` (for `COLL`) appeared in earlier versions of this table. They are not recognized by the v0.3.2 flag parser; see [COLL](#coll).
#### 3.1.4.2. Flag types
##### Round/Sign
Rounding and sign (absolute/negative) configuration string.  
Accepted Values:
- Optional round mode specifier followed by an optional sign mode specified without any spacing between the two.
- Examples:
	- `round+`: round and absolute.
	- `-`: negate, but don't round in any way.
	- `ceil`: round up to nearest integer, but don't modify the sign. 
- The sign mode specifier must be after the round mode specifier if both are given.
- An unrecognized round mode is not an error: it is treated as "no rounding". Double-check the spelling if a mode seems to have no effect.

Round mode specifiers:
- `round`/`r`: round to nearest integer
- `ceil`/`c`: round up to nearest integer
- `floor`/`f`: round down to nearest integer

Sign mode specifiers:
- `+`: force positive value (absolute)
- `-`: force negative value (negative absolute)
##### Float
Floating point number. Accepts any number that is not NaN or +/-infinity. Aliases are not accepted.
##### Boolean
`true` or `false`. Must be written as such (lowercase).
##### Operator
One of the four arithmetic operators: `+`, `-`, `*`, or `/`.
##### Dict
A dictionary delimited by braces, with `key=value` pairs separated by commas. Written like:
- `remap:{123=456}`
- `remap: {1=2, 3=4, ...}`
- `remap: {mem_8bit::mem_end = mem_8bit::cread}`

See [Supported flags](#3141-supported-flags) for the accepted keys/values and spacing rules.

### 3.1.5. Concurrent instructions
Concurrent instructions are denoted with a `~` prefix to their identifier. They are placed to be executed on the same tick as the previous instruction.
```
sequential:
	; executed one-by-one. takes 3 ticks.
	MOV C1, 1
	MOV C2, 2
	MOV C3, 3

concurrent:
	; all executed on the same tick. takes 1 tick.
	; not necessary to put a ~ on the first instruction
	MOV C1, 1	;                          	<-----+
	~MOV C2, 2	; executed on same tick as above -|
	~MOV C3, 3	; executed on same tick as above -+
```

> While having great potential to speed up any program that does not need a strictly sequential flow, the order in which instructions are executed is **NOT GUARANTEED TO BE THE SAME ORDER THAT THEY ARE WRITTEN**. This is due to the GD runtime executing triggers in different orders in the same tick in different cases, and not a fault of the compiler.  
> Please keep this in mind when writing a program that is order-sensitive. 

#### 3.1.5.1. Delays
A concurrent instruction is placed in the same space as the instruction before it, and the cluster then takes as long as its **last** instruction. In other words, the execution time of an instruction is overwritten if there is another concurrent instruction after it:
```
example1:
	WAIT 3          ; 3-tick instruction
	~MOV C1, 1      ; 1-tick instruction
    ; in this case, the instruction cluster has a 1-tick delay, since the last concurrent instruction in it is 1-tick.
    ; the WAIT is effectively cancelled.

example2:
    MOV C1, 1
    ~WAIT 3
    ; in this case, the cluster has a 3-tick delay, since WAIT 3 is a 3-tick instruction
```
This delay is always guaranteed and enforced by the compiler.

#### 3.1.5.2. Destructive operations
It is important to be mindful of the usage of destructive operations with concurrent instructions, as data may be overwritten non-deterministically. In the case of swapping two items, the following is a baseline implementation:
```
swap:
    MOV C3, C2  ; move to temp counter
    MOV C2, C1  ; previous value is overwritten
    MOV C1, C3  ; stored value is written back
```
This implementation is standard in regular procedural languages, however it may be made faster in the context of TASM with concurrent instructions:
```
swap:
    MOV C3, C2
    ~MOV C2, C1
    ~MOV C1, C3
```
This implementation theoretically performs the swap in a single tick. However, since the order of execution is not strictly guaranteed to be the same as is listed in the program, this operation is unsafe and may overwrite previously stored data.  
Therefore, we must be careful to not use possibly stale data and overwrite data that may not have been transferred in the same tick. In this case, we should not attempt to store the value of C2 and overwrite it with the value of C1 in the same tick.  
The following is the updated implementation, which runs in 2 ticks instead of 3:
```
swap:
    MOV C3, C2  ; executed on one tick
    MOV C2, C1  ; executed on the next
    ~MOV C1, C3
```

> [!NOTE]
> It is not clear what instructions work when made concurrent and what instructions do not. It is deterministic but largely unpredictable. Please contact the maintainer of this project if you have knowledge concerning the spawn order of objects.

## 3.2. Routines 
### 3.2.1. Routine declaration 
A routine is declared as such:
```tasm
routine_name: 
	INSTRUCTION
	... 
``` 
The routine identifier line must not be indented, must not contain spaces, and must end with a colon (that is not part of the identifier). Any other unindented line is a compile error (E0029).  
All instructions under that identifier that are indented will be considered part of that routine.  
Each routine name may only be declared once per file (error E0028).

> [!NOTE]
> Choose routine names that cannot be mistaken for another value. A routine named like an item, group or collision block literal (e.g. `c5`, `g12`, `t1`, `b3`) or a number (e.g. `inf`) will be parsed as that value instead of the routine when it is referenced.

Routines that contain no instructions are skipped and do not use a group.
### 3.2.2. Special routines 
Special routines are hard-coded to the compiler, and have special behaviour. They are *not* automatically generated. 
#### 3.2.2.1. `_start` routine
This routine is considered the entry point of the program, and is required by the compiler to be included in the input file unless the `--no-entry-point` argument is passed (error E0011 otherwise). 
This routine should be treated like the `main` function of other languages - the program is intended to start when this routine is spawned.
An [IOBlock](#121-ioblock) labelled `start` is automatically placed at IOBlock position 0 to activate the group assigned to this routine. Its spawn trigger is spawn-ordered unless `--unordered-start` is passed.
#### 3.2.2.2. `_init` routine
This routine is intended for all preliminary setup instructions not necessarily related directly to the program's function. For example, importing external modules, defining aliases, or marking items as persistent. 
The programmer should use this routine only for init-specific instructions. Though it is possible to include instructions such as `MOV` which are repeatable actions, it is not recommended to do this
since instructions in this routine are not preserved when the module is imported.

The `_init` routine does not use a group, and it cannot be spawned or referenced by other instructions (error E0033).  
Any non-initializer instruction found in the `_init` routine will be placed in the negative-x and positive-y quadrant, without a group. These triggers are activated once when the level starts. See specifics of each init instruction [here](#3131-initializer-instructions).  
To compile a program without placing the `_init` triggers (for example, when they are already in the level), pass `--skip-init`.
### 3.2.3. In-level object representation 
Apart from the `_init` routine, all routines are compiled individually by instruction, with each object cluster being placed in its own [space](#124-space), starting at x=105.
The width of a space depends on the compilation mode:
- **Release mode** (`--release`): each space is 1 unit wide. Instructions are packed as tightly as possible and execute one tick apart.
- **Default (debug) mode**: each space is 30 units (one grid block) wide, which makes the routine readable in the editor. As spawn ordered triggers are activated according to their x-position, this also means that each space takes 30 ticks (1/8 of a second) to execute.

All routine groups are also on separate lines from each other (30 units apart, ordered by group ID), annotated by text formatted as `group: routine`, and positioned on x=0 and the same y-level as the rest of the group. This text object is not a part of the routine group.   
This is done to ensure sequential execution with a spawn-ordered trigger.

Example (release mode):
```
routine:
	INSTRUCTION1  ; starts at x=105
	INSTRUCTION2  ; this is at x=106
	... ; and so on
```
### 3.2.4 Recursion
Routines have the ability to call themselves, under the condition that the instruction that calls the routine from within itself is not the first logical instruction.  
Be careful about using recursion with routines that have code after they call themselves.
In the following example:
```
routine:
	MOV C1, C2 				; do something
	FL routine, cont, C2, 0 ; call this routine recursively
	PAUSE routine			; wait for this routine to be unpaused for further execution
	ADD C3, 1
	RESUME routine			; resume next instance
cont:
	; unpause routine
	RESUME routine
```

This is dangerous, because the GD runtime does not have a call stack (of course, it is possible to make one in TASM). Therefore, upon unpausing `routine`, all paused instances of `routine` get released at once, and it is impossible to release all instances of `routine` one at a time, sequentially. 
## 3.3. Types of values 
Each argument is parsed into exactly one type. When a value could be read in more than one way, the first matching rule below wins:
1. Anything starting with `\` is a [string](#336-strings).
2. Anything containing `::` is a reference to an [external symbol](#341-using-external-symbols).
3. [Built-in aliases](#335-aliases) (`ATTEMPTS`, `POINTS`, `MAINTIME`, `COLL_P1`, `COLL_P2`, `COLL_P_ANY`).
4. [Collision blocks](#334-hitboxes) (`b5`).
5. [Item](#332-item-literals) and [group](#3331-group-literals) literals (`C5`, `T5`, `g5`).
6. [Number literals](#331-number-literals) (`5`, `2.5`, `0x1F`).
7. Otherwise, the value is a [routine](#3332-routines) if a routine with that name exists, and a [string](#336-strings) if not.

User-defined [aliases](#3128-the-alias-instruction) are substituted before any of these rules are applied.
### 3.3.1. Number literals
A number literal is any string that may be parsed as a float. Unless specified to be strictly an integer, all numbers are parsed as double-precision floats (f64).  
It is important to make the distinction between number literals and numbers stored in items. While both are numbers, number literals are used more as specific values, while items represent containers for values in the actual program/level.  
It is also important to recognize that all floats in GD are 32-bit floats. This means that any integer values above 2^24, or 16 777 216, while correctly parsed by the compiler, may be incorrectly rounded by GD itself.

Where an instruction requires an `<int>`, the number must have no fractional part (`5` and `5.0` are both accepted, `5.5` is not).

The compiler uses `input_string.parse::<f64>()` to parse a number literal, which supports these numbers (case-insensitive):
- Infinity: `inf`, `infinity`. Represents values higher than the maximum representible `f64`, `1.7e308`. 
- NaN: `nan`. Represents a value that is not a number (for example, the result of `0 / 0` in floating-point arithmetic). Note that dividing by zero in GD does not actually yield NaN, but sets the quotient to zero instead.
- Scientific notation such as `1e5` or `2.5E-3`.

#### 3.3.1.2. Hex literals
A hexadecimal integer literal may be written starting with the prefix `0x` (lowercase), followed by a number that will fit into a 32-bit signed integer. Any value that starts with `0x` will be parsed as a hexadecimal literal, and will emit an error (E0020) if it is not parsed instead of falling back to being parsed as a different type of value.
If a value starts with `0x` is intended to be a string, prefix it instead with `\`. 
### 3.3.2. Item literals
An item literal represents a GD item, most commonly a counter or timer item. It is denoted as such:
- Counter: `CXXXX`, where `XXXX` represents the ID of the counter. Example: `C123` represents the counter with ID 123.
- Timer: `TXXXX`, where `XXXX` represents the ID of the timer. Example: `T456` represents the timer with ID 456.

The prefix is case-insensitive (`c123` is the same as `C123`). IDs do not have to be 0-padded, and they must be in decimal form. They are only valid if they are in the range [1, 9999]. The same goes for IDs in group literals. The compiler rejects IDs of 0 or below (error E0017).  
Item literals are parsed by first checking for a prefix of either `C` or `T`, and if this is true, the rest of the literal is parsed as a base-10 signed 16-bit integer, since IDs are internally represented as signed 16-bit integers by GD. If the rest of the literal is not a number (e.g. `counter`), the value is not an item.

As of v0.3.2, item literals may also be given a hexadecimal ID: `Cx56` corresponds to counter with ID 0x56. This works for both counters and timers.

The built-in items `ATTEMPTS`, `POINTS` and `MAINTIME` are also items. See [Aliases](#335-aliases).
### 3.3.3. Groups
Both of the following are interally groups:
#### 3.3.3.1. Group literals
Group literals refer to a static group ID. They are written as `g{id}`, where ID is a valid group ID.  
Group literals are parsed the same way as item literals, except for the prefix (`g` or `G`).  
Example: `g123` refers to the group with ID 123.

As of v0.3.2, group literals may also be given a hexadecimal ID: `gx56` corresponds to group with ID 0x56.
#### 3.3.3.2. Routines
Routines are specified simply by their identifier. Since they are parsed first, any routine name declaration/reference order conflicts are avoided.
```
routine:
	; do whatever in here

spawner_routine:
	; spawn the routine here
	SPAWN routine
```
If the `--group-offset` argument is specified, the groups of each routine will change, which is unlike group literals, since they are static.  
Routines from imported modules are referenced with `module::routine`.
### 3.3.4. Hitboxes
Used for collision block instructions.
There are two ways to declare a hitbox:
1. `b{id}` where `id` is the decimal ID of a collision block (e.g. `b12`)
2. [Using a built-in alias](#335-aliases) for the player(s).

#### 3.3.4.1. Valid hitbox combinations
When using a collision instruction, the two hitboxes can only be one of the below pairs, in this order.
Any other combination is not supported in GD and will cause the compiler to throw an error (E0048).

| Hitbox 1                  | Hitbox 2        |
| ------------------------- | --------------- |
| Collision block           | Collision block |
| Player 1 (`COLL_P1`)      | Collision block |
| Player 2 (`COLL_P2`)      | Collision block |
| Either player (`COLL_P_ANY`) | Collision block |
| Player 1 (`COLL_P1`)      | Player 2 (`COLL_P2`) |

"Collision block" refers to a collision block with an ID - these can be declared with `b{id}` syntax. 


### 3.3.5. Aliases
Aliases act as substitutions for other values, namely, other items. They are used primarily to reference items that may not have a constant value.

<!-- Version number -->
As of TASM v0.3.2, these are the built-in aliases (they are case-sensitive and must be written in uppercase):
- `ATTEMPTS`: refers to the `Attempts` counter. Read-only.
- `POINTS`: refers to the `Points` counter.
- `MAINTIME`: refers to the `MainTime` timer. Read-only.
- `COLL_P1`: enables the option to check for collision with player 1 (for collision instructions)
- `COLL_P2`: enables the option to check for collision with player 2 (for collision instructions)
- `COLL_P_ANY`: enables the option to check for collision with player 1 or player 2 (for collision instructions)

User-defined aliases are created with the [`ALIAS` instruction](#3128-the-alias-instruction).

### 3.3.6. Strings
A string may be denoted with the escape character `\` to designate it as a string literal where it may otherwise be parsed as a value of a different type. For example, `g123` will compile to Group 123; however, 
`\g123` will compile into the string literal `"g123"`.   
If a value was not parsed as any of the above, it is left as a string. Strings are rarely used in the language, but a notable use is as a label for an IOBlock.  
**Note: Since strings are the fallback, values that maybe be interpreted as another type are NOT parsed as strings. Please be mindful of this when trying to pass a string argument which may, for example, look like a group or number, and thus will get parsed as that type if not escaped.**  
The `\` escape does not prevent a string from being resolved as a routine: a value that is exactly the name of a routine in the file is always treated as that routine.
### 3.3.7. Argsets 
Instructions may have different uses depending on the provided arguments. For this reason, they are explicitly typed. 
Since instruction arguments are typed, these types are checked during compilation in the [instruction parsing stage](#53-instruction-parsing). If no argset of the instruction matches the given arguments, the compiler emits error E0027 (or E0042 if the instruction references imported symbols), listing the types that it received.

The types used in this documentation are:

| Type | Accepts |
| ---- | ------- |
| `<item>` | Counter or timer literals, `ATTEMPTS`, `POINTS`, `MAINTIME` |
| `<timer>` | Timer literals only |
| `<number>` | Number literals |
| `<int>` | Number literals with no fractional part |
| `<group>` / `<routine>` | Group literals, routines (local or `module::routine`) |
| `<hitbox>` | Collision block literals, `COLL_P1`, `COLL_P2`, `COLL_P_ANY` |
| `<string>` | Any value that is not parsed as another type |

## 3.4. External symbols and the module system
The module system exists to allow for the reuse of code without needing to duplicate it to paste into the program. Modules may be imported with the usage of the [`IMPORT` instruction](#31210-the-import-instruction).

Due to not being able to import triggers, all imported routines are statically linked into the original program. Names are automatically mangled by the compiler, so there is no need to try to account for name conflicts.

Across files, the compiler internally mangles each symbol by joining the file's path with the symbol name, guaranteeing a globally unique internal name regardless of how many files reuse the same local identifier (e.g., two different libraries can each define a routine called multiply without conflict). This mangling is an internal linker detail - authors always refer to symbols with the module::symbol syntax and never see or need to construct the mangled form themselves in TASM. Note that mangled symbol labels are inserted into the generated level instead of the original identifier of that symbol.

### 3.4.1. Using external symbols
When importing a module, the objective is to use symbols defined in that module. This is done by importing the module wherein the desired symbol is defined and then referencing that symbol in the main program.
An external symbol is declared with the syntax `module::symbol`, where `module` is the file name (without extension) of the module where the `symbol` lives. This symbol can be either a routine or an alias.

External symbols may be used as instruction arguments and as keys/values in [Dict flags](#dict).

A library file may define its own `_init` routine. When that file is imported elsewhere, only its `ALIAS` and `IMPORT`/`IMPORTSTD` declarations take effect - those contribute aliases and further nested imports to the importing program, as shown above. Any other instructions in the library's `_init` (such as `PERS`, `DISPLAY`, `IOBLOCK` or `EVENT`) are ignored entirely when the file is used as an import; they are only meaningful if that file is compiled directly as the program's entry point.

Additionally, library files are not expected to have an entry point. They serve as collections of useful consts/functions that other files can import. The `_start` routine is only necessary when the file is compiled directly. 

### 3.4.2. Module usage examples 
#### Basic usage example
Library `library`:
```x86asm
multiply:
	mul C1, 2
```

Main program:
```x86asm
_init:
	import library

_start:
	mov C1, 4	; prep value
	spawn library::multiply
	; C1 is now 8
```

Here, the routine `multiply` is imported into the main routine from `library`. When this program is compiled, the routine will be copied into the main program. We can see this by enabling the `--linker-output` flag for the compiler:
```
; ------- linker output -------
_init: (0)
_start: (1)
    MOV Counter(1), Number(4.0) |
    SPAWN Group(2) |
<path>\library.tasm::multiply: (2)
    MUL Counter(1), Number(2.0) |
; ----- end linker output -----
```
The reference to `library::multiply` is replaced with its assigned group.

Import resolution is lazy: the linker only parses and links symbols that are actually reachable from the program. An imported module that is never referenced is never parsed at all.

#### Nested imports
Suppose we have a library file which uses another dependency:
```x86asm
; math.tasm needs consts.tasm
_init:
	import consts

; convert T1 from degrees to radians
to_radians:
	mul T1, consts::HALF_PI
	div T1, 90
```
`consts.tasm` definition:
```x86asm
_init:
	alias HALF_PI, 1.5708
```
Main program:
```x86asm
_init:
	import math

_start:
	mov T1, 90
	spawn math::to_radians
```

Both the routine `to_radians` will be inserted, as well as the value for `HALF_PI`. Dependencies are scanned recursively. If the imported module has no dependencies, it will be imported as-is. If it does, its dependencies will be resolved first and then imported.

The linker yields the following:
```x86asm
; ------- linker output -------
_init: (0)
_start: (1)
    MOV Timer(1), Number(90.0) |
    SPAWN Group(2) |
\\?\C:\Users\user\Documents\GitHub\tasm-lang\rtasm\math.tasm::to_radians: (2)
    MUL Timer(1), Number(1.5708) |	; <-- was `consts::HALF_PI`
    DIV Timer(1), Number(90.0) |
; ----- end linker output -----
```

### 3.4.3. Edge cases
Diamond dependencies are supported in TASM. The linker uses a module cache to keep track of all seen modules which also prevents any duplicate parsing and allows for faster compilation times.
The linker's module cache ensures each module's code is linked into the output exactly once, no matter how many other modules import it. If both `A` and `B` import `C`, `C` is only copied into the final program a single time - not once per importer.
If the linker encounters a circular chain of imports, it raises a compile error (E0045) rather than attempting to resolve the cycle (which would lead to infinite recursion). Prefer extracting the shared symbols both sides need into a common module that each imports independently, rather than having two modules import each other.

## 3.5. Group usage 
Some instructions use a group or two to sustain the intended functionality of the instruction. Please be mindful of this when working on a project that may take a lot of groups.
Below is the specification for all instructions and how many extra groups are used. Every routine (except `_init`) also uses one group of its own.

| Instruction                    | Groups      | Usage                                                                                  |
| ------------------------------ | ----------- | -------------------------------------------------------------------------------------- |
| Any arithmetic + MOV           | 0           | none                                                                                   |
| Spawn compare                  | 1           | Spawn trigger for group                                                                |
| Fork compare                   | 2           | Spawn triggers for both groups                                                         |
| `SRAND`                        | 1           | Spawn trigger for group                                                                |
| `FRAND`                        | 2           | Spawn triggers for both groups                                                         |
| Instant spawn/fork compare (incl. `ISRAND`/`IFRAND`) | 0 | This version does not use intermediate triggers.                             |
| SPAWN                          | 0           | none                                                                                   |
| Wait instructions (e.g. NOP)   | 0           | none                                                                                   |
| All other instructions         | 0           | none                                                                                   |

After compiling, the compiler prints the range of groups assigned to routines (e.g. `Using groups 1 - 5`). The extra groups used by instructions are not included in this range.

## 3.6. Comments
A comment is anything that follows a semicolon (`;`) on the same line.
## 3.7. Execution model
The execution model of TASM is one fairly similar to that of real hardware:
- All instructions take some amount of time to execute, always an integer amount of ticks (in release mode).
- Each group is assigned a primary group to start, though more are used per comparison instruction.
- Instructions are executed sequentially, and are placed from left to right when compiled to a level.
- Routines are spawned with spawn-ordered enabled by default.
- Spawned routines execute concurrently, no matter how many of them there are.
# 4. TASM Toolkit
## 4.1. Installing TASM
As of v0.3.2, there are install scripts for the TASM compiler (`tasmc`). There are two versions, one for windows, which is a powershell script, and one for linux, which is a shell script: 
- [Windows installer](https://tasm.mntpoint.org/install.ps1) (`install/windows.ps1` in the repository)
- [Linux installer](https://tasm.mntpoint.org/linux.sh) (`install/linux.sh` in the repository)

Both scripts take an action as the first argument:
- `install <version>`: downloads the given release (e.g. `v0.3.2`) from the GitHub releases page, installs it, and adds the installation directory to `PATH`. If a version is already installed, the script asks for confirmation before replacing it.
- `remove`: removes the installed version and its `PATH` entry.

```powershell
# Windows (PowerShell)
.\windows.ps1 install v0.3.2
```
```bash
# Linux
./linux.sh install v0.3.2
```

During installation, the script asks which build to install:
- Windows: MSVC (`x86_64-pc-windows-msvc`) or GNU (`x86_64-pc-windows-gnu`). Use MSVC if you are unsure, and GNU if you are not able to use MSVC.
- Linux: glibc (`x86_64-unknown-linux-gnu`) or musl (`x86_64-unknown-linux-musl`, statically linked, no glibc dependency).

TASM is installed to `%APPDATA%\tasm` on Windows and `~/.local/bin/tasm` on Linux. Restart your shell afterwards for the `PATH` change to take effect.

You may also download the pre-built executables from the [releases page of the GitHub repository](https://github.com/ArrowSlashArrow/tasm-lang/releases), however, if it is not possible to use them, refer to the below instructions for manually operating the compiler:
### 4.1.1. Building the rtasm compiler from source
Prerequisites: 
- Rust version v1.88.0 or later

In the `rtasm` directory of the project, run `cargo build --release` to compile the executable. Assuming a successful compile, the executable will be at `target/release/tasmc[.exe]`. 
### 4.1.2. Compiler options
Usage: `tasmc [OPTIONS] <INFILE>`

By default, the compiled level is added to your local Geometry Dash savefile as a new level (named after the input file). Close Geometry Dash before compiling, otherwise the game may overwrite the savefile when it exits. If the savefile cannot be found, use `--gmd` instead. Compiled levels have platformer mode enabled.

| Option | Description |
| ------ | ----------- |
| `-r`, `--release` | Compile in release mode: instructions are packed 1 unit apart so that they run as fast as possible, at the cost of readability in the editor. See [In-level object representation](#323-in-level-object-representation). |
| `-g`, `--gmd` | Export the level as `<level name>.gmd` instead of adding it to the savefile. |
| `-c`, `--clipboard` | Copy the compiled objects to the clipboard instead of exporting a level. They can be pasted in with BetterEdit. Windows only. |
| `--wslive [PORT]` | Send the compiled objects to the WSLive mod over a local websocket. |
| `--level-name <STRING>` | Name of the exported level. Defaults to the input file name. |
| `--group-offset <N>` | Start assigning routine groups after group `N` (the first routine uses group `N + 1`). Default: `0`. |
| `--no-export` | Compile the program but do not export it anywhere. Useful for checking a program for errors. |
| `--no-entry-point` | Do not require a `_start` routine. Useful for compiling libraries and utility programs. |
| `--skip-init` | Parse, but do not place, the triggers of the `_init` routine. Useful when they are already in the level. |
| `-u`, `--unordered-start` | Disable spawn ordered on the spawn trigger of the `start` IOBlock. |
| `-l`, `--linker-output` | Print the program after linking (all routines, their groups, instructions and flags). Used primarily for debugging. |
| `-D`, `--dependencies` | Print the paths of all modules that the program depends on. |
| `-v`, `--verbose-logs` | Enable verbose logging from the compiler. |
| `--no-log` | Disable all output from the compiler, including verbose logs. |
| `-e`, `--error-help <CODE>` | Print the documentation for an error code, e.g. `tasmc -e 27` for E0027. |
| `-d`, `--decompile` | Decompile the given `.gmd` file (or raw object string file) into TASM and print it to stdout. Experimental. |
| `-h`, `--help` / `-V`, `--version` | Print help / version information. |

Examples:
```bash
tasmc program.tasm                     # compile and add to the savefile
tasmc program.tasm -r -g               # release mode, export to program.tasm.gmd
tasmc program.tasm --level-name "My Level" --group-offset 100
tasmc library.tasm --no-entry-point --no-export   # check a library for errors
tasmc -e 31                            # explain error E0031
```
## 4.2. pytasm compiler
**NOTE:** pytasm is currently deprecated, and will NOT receive future updates. It is *HIGHLY* recommended to use the rust compiler instead. pytasm does not support most features added since its deprecation (including flags, imports and the newer instructions).  
**WARNING**: pytasm will __**OVERWRITE**__ the first level in your savefile. Please be mindful of this when compiling a program. 

Prerequisites: 
- Python 3.9 
- All packages in requirements.txt installed 
	- If not installed, run `pip install -r requirements.txt`.

Navigate to the `old/pytasm/` directory, and run `python main.py <program>.tasm` to compile the program. 
To see options, run `python main.py --help`. 
## 4.3. The interpreter/emulator 
Note: The interpreter is legacy software, and is currently only accessible through the deprecated pytasm compiler. Its source is located in `old/interpreter/`.

The interpreter is a tool which is designed to emulate the program in the context of the GD runtime. It is intended to provide developers with a way to debug their program without having to run it in GD every time to test it. 
It does not emulate the actual GD environment, which may involve niche edge cases and other unforeseen bugs. The interpreter program itself is written in rust, which makes it available for use on all operating systems. However, the only way to access it properly is through the pytasm compiler, which is built for windows and may break on linux.  

To access the interpreter, first navigate to the `old/pytasm/` directory. Then, run `python main.py <program>.tasm --interpret`.
## 4.4. Getting started
It may be intimidating to use a language like this one, however, the language is intended to be easy to read and understand. While the language is verbose, it should not be considered unapproachable in any way.

A typical workflow looks like this:
1. Write a `.tasm` file with a `_start` routine.
2. Close Geometry Dash and run `tasmc <file>.tasm` (add `-r` for release mode once the program works).
3. Open the newly added level in the editor, enter playtest, and hit the `start` IOBlock to run the program.
## 4.4.1. Example programs
Example programs can be found in the `example_programs/` directory in the repo, and the standard library in `stdlib/` contains further examples of valid TASM. Here are some of them:
#### Simple Arithmetic

```
_start:
	MOV C1, 0  ; initialise C1
	ADD C1, 1  ; add 1 to it
	MUL C1, 2  ; multiply it by 2
```
The code snippet above uses one group for the routine, and generates three objects, one for each instruction. This program takes 3 ticks to execute, since each instruction is a 1-tick.
#### Fibonacci Sequence
``` tasm
_init:
	; ensure that stdlib is downloaded and in the same directory as the program
    import stdlib/mem_8bit
	alias MAX_INDEX, 50
    alias temp_counter, c260

fib:
    ; read the previous value
    ; all `wait` instructions are necessary for the program to work with `--release`
    spawn mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait 2
    add mem_8bit::ptrpos, 1
    wait 2
    mov temp_counter, mem_8bit::cmemreg	; read from counter register
	wait 2

    spawn mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait 2
    add mem_8bit::cmemreg, temp_counter ; add next number to counter
    wait 2

    ; write the sum into the next memory cell
    add mem_8bit::ptrpos, 1
    wait 2
    spawn mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cwrite}
    wait 1
    
    ; move pointer back to the previous number in preparation for the next iteration
    sub mem_8bit::ptrpos, 1
    
    sl fib, mem_8bit::ptrpos, MAX_INDEX
  
_start:
    mov C2, 1   ; starting value
    mov mem_8bit::ptrpos, 1
    spawn fib
```
This program generates the fibonacci sequence in the provided memory. The result memory reads as such: 0 1 1 2 3 5 8 13 ...  
This program uses 3 groups of its own: one for the `_start` routine, one for the `fib` iteration routine, and one for the condition check at the end of the `fib` routine. The routines linked in from `mem_8bit` use further groups. It notably does not use a group for the `_init` routine, since the `_init` routine never uses a group.
#### Prime Checker

```
_init:
    DISPLAY C1 ; input value
    DISPLAY C2 ; check factor
    DISPLAY C3 ; max factor
    DISPLAY C4 ; auxiliary mod var
    DISPLAY C5 ; 1 = prime, 2 = not prime
  
next_iteration:
    ADD C2, 2
    
    ; mod C1 by C2 (the check factor), and store the result in C4
    FLDIV C4, C1, C2
    MUL C4, C2
    SUB C4, C1
    ; if C4 == 0, then the input is cleanly divisible by the current factor, and is therefore not prime.
    FE not_prime, loop_checker, C4, 0
  
loop_checker:
	; if the C3 (max factor) >= C2 (current check factor),
	; spawn another iteration. otherwise, since the not_prime routine has not been spawned yet,
	; declare the input prime. 
    FGE next_iteration, prime, C3, C2
  
not_prime:
    MOV C5, 2
  
prime:
    MOV C5, 1
  
_start:
    MOV C1, 997 ; setup values
    MOV C2, 1
    
    ; set max factor to be checked to input/2
    DIV C3, C1, 2
  
    ; c4 = c1 % 2
    FLDIV C4, C1, 2
    MUL C4, 2
    SUB C4, C1
    ; declare that the number is not prime, since the input is cleanly divisible by 2, and is therefore even.
    FE not_prime, next_iteration, C4, 0
```
This program checks whether the input value in C1 is prime. If so, it returns 1 in C5, otherwise it returns 2. It uses a total of 11 groups: 5 for routines, and 6 for comparisons.

> [!NOTE]
> `DIV C3, C1, 2` and `FLDIV C4, C1, 2` use the `<item> <item> <number>` form, which is affected by the [known issue](#3121-arithmetic) in v0.3.2. Append `| divmod:true` to both of these lines for the program to behave as described.

# 5. Compiler spec 
This section is intended for advanced users and/or contributors. It is not necessary to read to use TASM.  
Note: this section is an overview of the compiler, and omits some details. To resolve any ambiguity, please read the compiler source code comments.  
The compiler executes the following sections in order:
## 5.1. Preprocessing 
Before anything other processing is done to the source code, some preprocessing is applied to it. 
The steps are as such: 
1. All tabs are converted to spaces
2. The source code is split into lines 
3. Each line is stripped of comments (everything from the first `;`) and of whitespace on the right
4. Blank lines are skipped by the following stages, but keep their line numbers for error messages
These steps are done to minimize any spacing and/or formatting errors, since this language is mostly formatting-insensitive. 
## 5.2. Routine indexing 
Before any instruction parsing, all routines are first indexed. This is important to resolve all routines before any are referenced in instructions, and possibly (incorrectly) determined to be invalid. 
Routines are parsed as such: 
1. For each line, 
	1. if the line is not indented, ends with a colon (`:`) and contains no spaces, it is considered a routine identifier. The current routine identifier is set to this identifier (but without the ending colon), and the next group is assigned to it (except for `_init`). Any other unindented line is a bad token (E0029).
	2. if the line is indented, it will be collected into a list of instructions associated with this routine
2. Routines are collected into a list of routine data: (routine starting line, routine identifier, routine group, routine lines with line numbers). Routines without any instructions are dropped and give their group back.
3. The `_init` routine is moved to the front, and its `ALIAS`, `IMPORT` and `IMPORTSTD` instructions are processed to build the alias table and the import list.
## 5.3. Instruction parsing 
If an instruction line is empty, it is skipped. Otherwise,
1. The line is split at the `|` character (if present). Everything after it is parsed as [flags](#314-instruction-flags).
2. The instruction arguments are parsed like so:
	1. The first space character is found, and anything to the left of it is considered the instruction identifier (with an optional `~` prefix for concurrency), and anything to the right of the argset.
	2. For `RAW` and `RAWTRG`, the argset is kept as a single string. Otherwise, the argset is split along each comma, and each argument is stripped of spaces on either side.
	3. Each argument that matches a user-defined alias is replaced with the alias's value.
	4. Each argument is parsed as a TasmValue, which may be one of the types listed [earlier](#33-types-of-values).
3. Next, the matching identifier's instruction sets and their handlers, and whether this is an initializer instruction is pulled from the instruction spec table. 
4. Check that this instruction is allowed in the routine if the instruction is exclusive to the initializer routine.
5. If any argument references a routine, the handler lookup is deferred to the linking stage. Otherwise, if the argset matches any set of types of that instruction, the respective argument handler function pointer and other relevant info (such as line number and type) is returned in an Instruction object. Otherwise, the parser throws an error.

## 5.4. Linking
1. Every reference to a symbol in another module (`module::symbol`) is resolved by locating the module in the import list by its file name, parsing it (recursively, through the module cache), and copying the referenced routine and all of the routines it depends on into the main program under a mangled name.
2. Imported aliases are substituted with their values, and Dict flags are resolved to integer pairs.
3. Every instruction whose handler lookup was deferred is matched against its argsets again, now that all routine references have known groups.

## 5.5. Compilation to level
At this point, we have a complete set of routines with valid instructions, so the compiler assumes this.
Instructions are converted to objects in this manner:
1. Keep track of the current group and related information
2. For each routine,
	1. Determine the y-position of the group and reset object position
	2. Check that the current group does not exceed the group limit of 9,999. Throw an error and exit the compilation process if it does.
	3. Place the `group: routine` text label (except for `_init`)
	4. For each instruction,
		1. Check that the instruction does not write to a read-only item
		2. If the instruction is concurrent, move the object position back by the spacing of the previous instruction
		3. Call the instruction handler function with the instruction's argset and flags
		4. Add the returned object(s) to the level
		5. Update any data returned alongside the objects, which may include: extra groups used, amount of spaces to skip (on the x-axis), whether a counter display was added, etc.
		6. increment the x-position of the next object cluster by the amount of spaces to skip, multiplied by the space width (1 unit in release mode, 30 units otherwise)
3. If the group of the entry point is not 0, i.e. that the entry point exists and has a group, add an IOBlock for it. 
## 5.6. Extended Backus-Naur grammar definition
Note: This grammar is **approximate**. It may allow some things that the compiler doesn't or overshadow details.
```ebnf
(* Note: This grammar is approximate. Argument/flag *types* are resolved
   semantically at compile time (see §3.1.2 argsets, §3.3.7), not by this
   grammar. Where an alternative below could match more than one production,
   the first matching alternative (top to bottom) wins, mirroring the
   documented value parsing order (§3.3). *)

program ::= { blank_or_comment_line } { routine } ;

routine ::= label { instruction_stmt | blank_or_comment_line } ;

label ::= identifier ":" line_end ;
(* Must start at the first column and contain no whitespace. *)

instruction_stmt ::= ws { ws } [ "~" ] instruction [ { ws } flags ] line_end ;
(* Instruction lines must be indented. *)

blank_or_comment_line ::= { ws } [ comment ] newline ;

line_end ::= { ws } [ comment ] newline ;

instruction ::= raw_instruction | import_instruction | normal_instruction ;

(* ---- RAW / RAWTRG: opaque payload, not tokenized as arguments ---- *)
raw_instruction ::= raw_mnemonic ws raw_object_string ;
raw_mnemonic ::= "RAW" | "RAWTRG" ;   (* matched case-insensitively *)
raw_object_string ::= { any_char_except_newline_semicolon_or_pipe } ;
(* Special-cased: unlike every other instruction, the object string is not
   split on commas into `argument`s — it is captured as a single opaque
   payload. Since `;` starts a comment, the payload ends at the first `;`,
   so only a single object can be given per instruction (§3.1.2.9). *)

(* ---- IMPORT / IMPORTSTD: filesystem-style path, not an identifier ---- *)
import_instruction ::= import_mnemonic ws import_path ;
import_mnemonic ::= "IMPORT" | "IMPORTSTD" ;   (* matched case-insensitively *)
import_path ::= path_segment { "/" path_segment } ;
path_segment ::= ".." | word ;

(* ---- everything else ---- *)
normal_instruction ::= mnemonic [ ws argument { { ws } "," { ws } argument } ] ;

mnemonic ::= identifier ;
(* Matched case-insensitively against the fixed instruction set in §3.1.2. *)

argument ::= escaped_string
           | qualified_identifier   (* external symbol, e.g. `mem_8bit::mem_end` *)
           | builtin_alias
           | hitbox
           | item
           | group
           | number
           | identifier             (* routine name or user alias *)
           | bare_string ;

builtin_alias ::= "ATTEMPTS" | "POINTS" | "MAINTIME"
                | "COLL_P1" | "COLL_P2" | "COLL_P_ANY" ;   (* case-sensitive *)

(* ---- flags (§3.1.4) ---- *)

flags ::= "|" { ws } flag { ws { ws } flag } { ws } ;

flag ::= scalar_flag | dict_flag ;

scalar_flag ::= flag_name ":" scalar_flag_value ;
(* No whitespace permitted between ':' and the value, per §3.1.4.1. *)

dict_flag ::= flag_name ":" { ws } dict ;
(* Whitespace between ':' and '{' is optional, but required if the first
   dict entry contains `::`, per §3.1.4.1. *)

flag_name ::= lowercase_letter { lowercase_letter | digit } ;   (* case-sensitive *)

scalar_flag_value ::= boolean
                     | number
                     | operator
                     | round_sign ;

boolean ::= "true" | "false" ;

operator ::= "+" | "-" | "*" | "/" ;

round_sign ::= [ round_mode ] [ sign_mode ] ;
round_mode ::= "round" | "r" | "ceil" | "c" | "floor" | "f" ;
sign_mode ::= "+" | "-" ;

dict ::= "{" { ws } dict_entry { { ws } "," { ws } dict_entry } { ws } "}" ;
dict_entry ::= dict_value { ws } "=" { ws } dict_value ;
dict_value ::= int_literal | hex_literal | item | group | collision_block
             | qualified_identifier | identifier ;
(* `identifier` here is a local routine or a local alias. *)

(* ---- values (§3.3) ---- *)

item ::= counter | timer ;
counter ::= ( "C" | "c" ) item_id ;
timer ::= ( "T" | "t" ) item_id ;
group ::= ( "g" | "G" ) item_id ;
item_id ::= decimal_id | hex_id ;
decimal_id ::= digit { digit } ;   (* semantically constrained to [1, 9999] *)
hex_id ::= ( "x" | "X" ) hex_digit { hex_digit } ;
(* Distinct hex convention from the standalone hex number literal below —
   `Cx56`/`gx56` have no leading "0". *)

hitbox ::= collision_block | "COLL_P1" | "COLL_P2" | "COLL_P_ANY" ;
collision_block ::= ( "b" | "B" ) decimal_id ;

number ::= decimal_number | hex_literal ;
hex_literal ::= "0x" hex_digit { hex_digit } ;   (* fits a 32-bit signed int *)
decimal_number ::= ["-" | "+"] ( digit { digit } [ "." { digit } ] | "." digit { digit } )
                    [ ("e" | "E") ["+" | "-"] digit { digit } ]
                 | "inf" | "infinity" | "nan" ;   (* case-insensitive *)
int_literal ::= ["-"] digit { digit } ;

escaped_string ::= "\" { any_char_except_comma_newline_semicolon_or_pipe } ;
bare_string ::= { any_char_except_comma_newline_semicolon_or_pipe } ;
(* fallback: only reached if nothing above matched *)

qualified_identifier ::= identifier "::" identifier ;

identifier ::= ( letter | "_" ) { letter | digit | "_" } ;
word ::= { letter | digit | "_" } ;

comment ::= ";" { any_char_except_newline } ;

ws ::= " " | "\t" ;
newline ::= "\n" | "\r\n" ;

letter ::= "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M"
         | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
         | lowercase_letter ;

lowercase_letter ::= "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m"
                   | "n" | "o" | "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" ;

hex_digit ::= digit | "a" | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F" ;

digit ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
```
