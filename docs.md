<!-- # 0. These docs are still Work-In-Progress! -->
# 1. Overview 
## 1.1. Abstract 
TASM is a powerful, domain-specific language that is designed to take advantage of the trigger system in Geometry Dash. The language is intended as an alternative to hand-placement of triggers in a level, and encourages developers to instead write clean code to achieve the same. 
The language is theoretically turing-complete, assuming unbounded IDs, however, GD imposes strictly unavoidable [constraints](#21-constraints). 
A powerful instruction set is provided, which allows for looping, branching, storage of information in memory, as well as a versatile SDK.

Quick links:
- [Available Instructions](#312-available-instructions)
- [Group Usage](#34-group-usage)
- [Special Routines](#322-special-routines)
- [Example Programs](#441-example-programs)
- [Types of Values](#33-types-of-values)
- [TASM Toolkit](#4-tasm-toolkit)
## 1.2. Terms and definitions 
### 1.2.1. IOBlock 
An IOBlock is a structure that consists of the following:
- A default block
- A multi-activate touchable spawn trigger for the corresponding group (to activate it)
- A text label, preferably one that describes the purpose of the IOBlock (except for the starting IOBlock)
IOBlocks are intended as a mechanism for the creator to test the functionality of the program in-level by hitting the block with your player.
### 1.2.2. Argset
Abbreviation for "Argument Set". Simply, a set of arguments passed to an instruction.
```
INSTRUCTION a, b, c
```
Here, `[a, b, c]` is the argset for the instruction.
### 1.2.3. n-tick
n-tick refers to the execution time of any single instruction. A 1-tick instruction takes exactly one tick to execute.  
To be clear, no instructions have a delay of execution. The execution time refers to how long the instruction takes to process.
### 1.2.4. MEMREG
MEMREG is an abbreviation for "Memory Register". It is also the alias for the memory register item in TASM. 
### 1.2.5. Memory mode/function
Memory mode is simply the mode of the memory. There are two modes:
- read mode: When MFUNC is called, the current memory cell's stored value is read to the MEMREG.
- write mode: When MFUNC is called, the value in the MEMREG is stored inside of the current memory cell.  

When a memory mode is set, its group is toggled on, and the other's is toggled off.
## 1.3. Version and updating

The version is defined according to [semantic versioning](https://semver.org).
### 1.3.1. Current version
<!-- Version number -->
The current version, as of April 12, 2026 is **v0.2.2**. 
Development of the project can be found on the [TASM repo](https://github.com/ArrowSlashArrow/tasm-lang).
# 2. The GD environment
This section contains documentation of the GD environment that is relevant to the purposes and function of TASM and/or the compiler.
## 2.1. Constraints
While TASM is theoretically turing-complete, assuming unbounded IDs, the GD environment imposes strict limits that are impossible to bypass. As such, TASM programmers must be aware of these constraints and their implications.
- IDs are integers in the range \[1, 10000). As a result, one may theoretically store up to 80KB of information, assuming the availability of each and every counter and timer. 
- Counter items (counters) are 32-bit integers. They may hold any value from \[-2<sup>32</sup> , 2<sup>32</sup>-1).
- Timer items (timers) are 32-bit floats, as per the [IEEE-754](https://en.wikipedia.org/wiki/Single-precision_floating-point_format) implementation.
- The game runs on a 240Hz loop, which means that 1 tick in TASM takes, in theory, exactly 1/240th of a second (~4.166ms). As a result, trigger programs are quite slow compared to real programs.

## 2.2. Useful mechanics
When compiled, the spawn trigger for every routine ALWAYS uses the spawn-ordered option. This is to ensure control of execution and pauses between instructions. If not enabled, the spawn will incorrectly skip waits and make every instruction 1-tick, which is undesirable since some instructions need downtime to be fully and correctly processed.

# 3. The TASM language 
## 3.1. Instructions 
### 3.1.1. Instruction syntax 
Instructions are written by their identifier followed by a space, followed by comma-separated arguments.
If an instruction takes no arguments, simply the instruction identifier is enough.
Examples:
```
INSTRUCTION argument1, argument2
NOARGS
```

It is important to know that instruction arguments (argsets) are typed to ensure differentiation between different functions of an instruction.
For example, the `SE` instruction is used as a branch instruction. It allows both the comparison of an item to a number and two items to each other:
```
SE example_routine1, C1, 0
SE example_routine2, C1, C2
```
Note that instruction argsets are typed to ensure that valid arguments are passed. Learn more in [this section](#335-argsets).
### 3.1.2. Available instructions 
All instructions in this section are expected to be fully functional. Any deprecated instructions will not be listed as of the next minor release.
#### 3.1.2.1 Arithmetic
All arithmetic instructions are 1-tick.  
By convention, the counter that stores the result of an arithmetic operation is usually specified as the first argument.
##### Argument format

| Argset                   | Result (in the example case of division) | Commands that use it           |
| ------------------------ | ---------------------------------------- | ------------------------------ |
| `<item> <number>`        | `item = item / number`                   | ADD, SUB, MUL, DIV, FLDIV, MOV |
| `<item> <item>`          | `1st item = 1st item / 2nd item`         | ADD, SUB, MUL, DIV, FLDIV, MOV\* |
| `<item> <item> <number>` | `1st item = 2nd item / number`           | MUL, DIV, FLDIV                |
| `<item> <item> <item>`   | `1st item = 2nd item / 3rd item`         | MUL, DIV, FLDIV, ADD, SUB      |

Note: Neither `MAINTIME` nor `ATTEMPTS` can ever be the result. Those items are immutable.
##### Instruction operations
- ADD: addition
- SUB: subtraction
- MUL: multiplication
- DIV: division
- FLDIV: division, and the result it rounded down (floored).
- MOV: assignment
> \* MOV simply assigns the 2nd item to the 1st item in this case. Data is not transformed when MOV is used.
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
#### 3.1.2.2. Compare
Spawning a group does not automatically pause the parent group.  
All compare instructions are 1-tick.  
Execution timeline:
- Tick n: the compare trigger is called
- Tick n + 1: 
	- the compare trigger calls the group spawner trigger if it should be called
	- the parent group executes the next instruction.
- Tick n + 2: the spawned group starts execution
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

##### Random Spawns
`SRAND <routine> <number>`

Spawns the first routine according to the supplied chance (2nd argument).  
The second argument (the chance) should be a float in the range \[0.0, 100.0].

`FRAND <routine> <routine> <number>`

Spawns the first routine according to the supplied chance (2nd argument).
If the first routine is not spawned, the second routine is spawned.  
The third argument (the chance) should be a float in the range \[0.0, 100.0].

Examples:
```
SRAND do_stuff, 42.8
```
The routine `do_stuff` has a 42.8% chance of being spawned.

#### 3.1.2.3. Memory
##### INITMEM
Arguments: `INITMEM <numbers>`

Assigns the numbers to memory in order, starting at address 0. Must be done after MALLOC. Numbers must be separated by commas, and should match the type of the memory (i.e. no floats in integer memory).  
Only allowed `_init` routine.
##### MALLOC
Arguments: `MALLOC <positive int>`

Allocates a specified amount of counters to memory. 
Only one memory allocation is allowed per program.  
Only allowed `_init` routine.
##### FMALLOC
Arguments: `FMALLOC <positive int>`

Allocates a specified amount of timers (floats) to memory. 
Only one memory allocation is allowed per program.  
Only allowed `_init` routine.
##### MFUNC
Arguments: `MFUNC`

If the current memory mode is read mode, then the value of the current memory location will be read to the MEMREG.
If the current memory mode is write mode, then the value of to the MEMREG will be written to the current memory location.  
Execution time: 2 ticks.  
##### MREAD
Arguments: `MREAD`

Sets the memory mode to read mode.  
Execution time: 1 tick.  
##### MWRITE
Arguments: `MWRITE`

Sets the memory mode to write mode.  
Execution time: 1 tick.  
##### MPTR
Arguments: `MPTR <int>`

Moves the pointer by a specified amount. A positive number pushes it forward into memory, while a negative number retracts it back towards address 0.  
Note: If the pointer is moved outside of memory, namely outside the range \[0, memsize), it will not read any memory, and will not get moved back down if MFUNC is called. 
Please be mindful of this when using the instruction. If it is desirable that the pointer returns to valid address space, please use the instruction MRESET.  
Execution time: 1 tick.  
##### MRESET
Arguments: `MRESET`

Resets the pointer position to 0.  
Execution time: 1 tick.  
##### 3.1.2.3.1. Memory safety guarantees
If the pointer is outside of the memory range \[0, memsize), no memory will be read. This means that nothing will be read from or written to the MEMREG, but the pointer will still move upwards.
If INITMEM is not called, the default values of each memory cell will remain, which are 0 for both counters and timers.
#### 3.1.2.4. Process
##### SPAWN
Arguments: `SPAWN <routine>`

Spawns the corresponding routine. Does not pause the current group.  
Execution time: 1 tick.  

##### PAUSE
Arguments: `PAUSE <routine>`

Pauses the specified routine via a stop trigger.
> A routine cannot be paused immediately after is it spawned. If it paused this way, the pause will simply be overshadowed by GD. A minimum of one tick is needed between the execution of the first non-wait instruction of the routine and the PAUSE for it to function. 
Execution time: 1 tick.

##### RESUME
Arguments: `RESUME <routine>`

Resumes execution of a paused routine via a stop trigger.  
Execution time: 1 tick.

##### STOP
Arguments: `STOP <routine>`

Stops the specified routine via a stop trigger. Unlike `PAUSE`, once a routine is stopped with this instruction, it cannot be resumed.
Execution time: 1 tick.

#### 3.1.2.5. Wait
##### NOP
Arguments: `NOP`

Does nothing on the tick it is called (equivalent to `WAIT 1`). Useful for waiting.  
Execution time: 1 tick.  
##### WAIT
Arguments: `WAIT <int>`

Does nothing for in following n ticks. Effectively a series of `NOP`s.  
Wait time cannot be negative. The compiler throws an error if it is specified as such. 

Execution time: variable.
#### 3.1.2.6. Time
##### TSPAWN
Arguments: `TSPAWN <timer> <float> <float> <routine>`

Starts the timer at the given time (2nd argument), and spawns the given routine when the timer reaches the target time (3rd argument).  
The timer can be written to during the period that it is ticking. This is not recommended for usage of timer items as timers.

Execution time: 1 tick.
##### TSTART
Arguments: `TSTART <timer>`

Unpauses the specified timer. To be started, the timer must have first been started by a time trigger via `TSPAWN`.  
Execution time: 1 tick.

##### TSTOP
Arguments: `TSTOP <timer>`

Pauses a running timer.  
Execution time: 1 tick.
#### 3.1.2.7. Miscellaneous
##### PERS
Arguments: `PERS <item>`

Makes the corresponding item persistent.  
Only allowed in the `_init` routine.
##### DISPLAY
Arguments: `DISPLAY <item>`

Adds a counter object for the corresponding item.  
Only allowed in the `_init` routine.
##### IOBLOCK
Arguments: `IOBLOCK <group> <int> <string>`

Places a block at the bottom of the level, at the specified x-position (2nd argument) with an annotation (3rd argument).
Also places a touchable spawn trigger that spawns the specified group.
Intended as a debug feature and/or substitute for user input.  

Only allowed in the `_init` routine.
#### 3.1.2.8. The `ALIAS` instruction
`ALIAS` is a special instruction that may only be used in the `_init` routine. It is used for defining custom aliases for values.  
For readability, the following can be rewritten, from: 
```
_start:
	MOV C1, 10
	MOV C2, 10
	MOV C3, 10
```
to:
```
_init:
	ALIAS start_value, 10

_start:
	MOV C1, start_value	; becomes MOV C1, 10
	MOV C2, start_value ; and so on...
	MOV C3, start_value
```

This instruction is parsed before any other instruction to build an alias reference table. This is done to be able to resolve the values of referenced aliases when determining types of argsets.  
Aliases are intended for developers to improve the readability of code that uses common values and reduce the usage of magic values.
Defined aliases are global and constant and cannot be re-assigned.  
The instruction takes two arguments as input: `ALIAS <string> <value>`.
* The string is the identifier of the alias and the identifier by which it must be referred to. Since aliases are parsed before anything else, aliases are resolved regardless of the location of their definiton within the `_init` routine.
* The value may be of any type. This value is what is inserted when an alias is referenced by its identifier.

```
_init:
	ALIAS value, 42

_start:
	MOV C1, value
	; becomes:
	MOV C1, 42
```

Aliases will not clone values from other aliases:
```
_init:
	ALIAS value, 42		; alias `value` holds 42
	ALIAS value2, value	; alias `value2` holds "value", NOT 42. 
```

### 3.1.3. In-level object representation 
All arithmetic instructions use a single Item Edit trigger, including MOV.  
All spawn compare instructions use 2 triggers: one for the Item Compare, to perform the comparison, and one for the group spawner.  
All fork compare instructions use 3 triggers: one for the Item Compare, to perform the comparison, and two for each group spawner.  

NOP does not compile to any objects, instead, a black space is left which acts as a wait since the group will be spawn-ordered.  
SPAWN simply adds a spawn trigger (with spawn-ordered enabled) to the specified group.   
> It should be noted that all group are spawned by a spawn trigger with spawn-ordered enabled.  

MFUNC, MPTR and MRESET are move triggers that target the memory pointer. MPTR and MRESET also include item edit triggers that update the pointer's position in the PTRPOS item.  
MREAD/MWRITE set the read mode by toggling on the respective item group and toggling off the other item group.  
- MREAD toggles on the read group and toggles off the write group
- MWRITE toggles on the write group and toggles off the read group

#### 3.1.3.1. Initializer instructions
All initiazlier instructions correspond to custom in-level structures, which may not necessarily be single triggers. For this reason, they are allowed only as setup instructions.
Below is a list of instructions and their corresponding structures:
- `MALLOC`: Creates a block of memory cells with a pointer collision block and a reset block, where each memory cell contains a:
	- collision block, for detecting the collision between it and the pointer, triggering the execution of the memory function,
	- item edit trigger for reading the value of this memory cell to the MEMREG, (on the read group),
	- item edit trigger for writing the value of the MEMREG to this memory cell (on the write group),
	- move trigger, for moving the pointer once the collision with this cell's collider is registered,
	- counter object, for a visual display of the current memory cell's value,
	- collision trigger, for registering the collision between the pointer and this cell's collider. This object is placed before x=0 so that it is initialised before anything.
- `FMALLOC`: Like `MALLOC`, except that all of the memory cells and the MEMREG are timers (floats), hence the `F` in `FMALLOC`.
- `INITMEM`: A column of item edit triggers that set each memory cell to the given values. Intended to initialise memory with values.
- `IOBLOCK`: An [IOBlock](#121-ioblock) that is put at y=75 and some specified x-position that acts as a debug group spawn. The x-position is processed such that it translates to a block position, e.g. 5 becomes 5 blocks (+ 2 for margin) to the right of the y-axis, centered on a cell.
- `PERS`: Adds a persistent item trigger for the specified item.
- `DISPLAY`: Displays a counter at some specified height and x=0 of the given counter.
### 3.1.4. Instruction flags
The function of a given instructions is usually simple/single-purposed, and only uses a handful of parameters within the trigger that it compiles to. However, triggers are remarkably configurable, and in some cases may simplify otherwise needlessly complex setups.  
A common example is the implementation of getting the absolute value of a number. The old implementation required a comparison of the target (C1) against 0 to determine its sign, which determined whether it should be negatied. This is much more complex and wasteful of groups than simply using the absolute rounding mode.
  
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
- Flags MUST be written after a `|` in the instruction line. There must only be one pipe character in the line if flags are used.
	- `ADD C1, C2 | itemmod:0.5` compiles.
	- `ADD C1, C2 | itemmod:0.5 | round:+` does not compile. 
- Flag-value pairs must be separated by whitespace. The flag identifier and its value themselves must be separated by a colon, but with no whitespace in between
	- `... | itemmod:0.5` compiles.
	- `... | itemmod: 0.5` does not compile.
	- `... | itemmod: 0.5, ` does not compile.
	- This only applies if the flag accepts a data type other than Dict. Dicts must be denoted as such:
		- `... | dict: {a:b, c:d, ...}`
		- There must be no spacing between the braces and the key/values.
		- There must be no spacing between the keys/values and the colon separator.
		- Key-value pairs must be separated by a comma. There may be whitespace after the comma.
		- There must be whitespace between the colon that separates the flag identifier and the value, and the dictionary itself: `dict: <whitespace> {...}` 

> [!NOTE]
> The types of values that flags accept are different to those listed in the [Types of Values](#33-types-of-values) section. Please refer to the [Flag types](#3142-flag-types) section for more info on accepted values for flags.

> [!NOTE]
> "Item result" refers to the intermediate result between the operands in an item edit trigger (used be arithmetic instructions) that is processed before any additional operations, such as usage of the multiplier or assignment to the target item.
> ![Item Result](./img/item_result.png)

| Flag    | Usage                                                                                                 | Instructions | Type       |
| ------- | ----------------------------------------------------------------------------------------------------- | ------------ | ---------- |
| resmode | Rounding and sign config for the item result                                                          | Arithmetic   | Round/Sign |
| finmode | Round and sign config for final computed result                                                       | Arithmetic   | Round/Sign |
| itemmod | Modifier in arithmetic instructions. Item result is multiplied by it by default.                      | Arithmetic   | Float      |
| divmod  | Divides item result by modifier rather than multiplying it.                                           | Arithmetic   | Boolean    |
| iter    | Compund assignment operator to target item. Akin to `+=`.                                             | Arithmetic   | Operator   |
| op      | Arithmetic operator between items. Does nothing if there are less than 2 input operands.              | Arithmetic   | Operator   |
| delay   | Spawn delay in seconds.                                                                               | `SPAWN`      | Float      |
| remap   | ID remap descriptor. Each key-value pair represents the old ID and the new ID respectively.           | `SPAWN`      | Dict       |
| tpaused | Starts target timer paused.                                                                           | `TSPAWN`     | Boolean    |
| tstop   | Stops target timer once the target time has been reached.                                             | `TSPAWN`     | Boolean    |
| tmod    | Time multiplier for timer. Can be negative.                                                           | `TSPAWN`     | Float      |
| nover   | Only activate if the target timer is not running, or it is at 0.00, or the `tpaused` flag is enabled. | `TSPAWN`     | Boolean    |
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

Round mode specifiers:
- `round`/`r`: round to nearest integer
- `ceil`/`c`: round up to nearest integer
- `floor`/`f`: round down to nearest integer

Sign mode specifiers:
- `+`: force positive value (absolute)
- `-`: force negative value (negative absolute)
##### Float
Floating point number. Accepts any number that is not NaN or +/-infinity.
##### Boolean
`true` or `false`. Must be written as such.
##### Operator
One of the four arithmetic operators: `+`, `-`, `*`, or `/`.
##### Dict
A dictionary delimited by braces, with key-value pairs separated by commas. Written like:
- `{123:456}`
- `{1:2, 3:4, ...}`

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

> While having great potential to speed up any program that does not need a strictly sequential flaw, the order in which instructions are executed is **NOT GUARANTEED TO BE THE SAME ORDER THAT THEY ARE WRITTEN**. This is due to the GD runtime executing triggers in different orders in the same tick in different cases, and not a fault of the compiler.  
> Please keep this in mind when writing a program that is order-sensitive. 

#### 3.1.5.1. Delays
Instruction execution times are overwritten if there is another concurrent instruction after it:
```
example1:
	SE g123, C1, 0	; 2-tick instruction
	~MOV C1, 1      ; 1-tick instruction
    ; in this case, the instruction cluster has a 1-tick delay, since the last concurernt instruction in it is 1-tick.

example2:
    MOV C1, 0
    ~SE g123, C1, 0
    ; in this case, the cluster has a 2-tick delay, since SE is a 2-tick instruction
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
> While there likely is a deterministic and predictable way in which triggers are executed in one tick, the maintainer of this project was not aware of the mechanism. If it becomes known, please open a feature request PR on the repo.

## 3.2. Routines 
### 3.2.1. Routine declaration 
A routine is declared as such:
```tasm
routine_name: 
	INSTRUCTION
	... 
``` 
The routine identifier line must not be indented and must end with a colon (that is not part of the identifier).  
All instructions under that identifier that are indented will be considered part of that routine.
### 3.2.2. Special routines 
Special routines are hard-coded to the compiler, and have special behaviour. They are *not* automatically generated. 
#### 3.2.2.1. `_start` routine
This routine is considered the entry point of the program, and is required by the compiler to be included in the input file.  
An [IOBlock](#121-ioblock) is automatically placed to activate the group assigned to this routine. 
#### 3.2.2.2. `_init` routine
This routine is intended for any preliminary setup instructions. For example, declaring and initializing memory with values.  
This is the only routine where initializer instructions are allowed, because they correspond to custom static structures in the level. See specifics of each instruction [here](#3131-initializer-instructions).
Any non-initializer instruction found in the `_init` routine will be placed in the negative-x and positive-y quadrant.
### 3.2.3. In-level object representation 
Apart from the `_init` routine, all routines are compiled individually by instruction, with each object cluster being separated by one unit on the x-axis, starting at x=105.
All routine groups are also on separate lines from each other, annotated by text formatted as `group: routine`, and positioned on x=0 and the same y-level as the rest of the group. This text object is not a part of the routine group.   
This is done to ensure sequential execution with a spawn-ordered trigger.

Example:
```
routine:
	INSTRUCTION1  ; starts at x=105
	INSTRUCTION2  ; this is at x=106
	... ; and so on
```
### 3.2.4 Recursion
Routines have the ability to call themselves, udner the condition that the instruction that calls the routine from within itself is not the first logical instruction.  
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

This is dangerous, because the GD runtime does not specify a call stack. Therefore, upon unpausing `routine`, all paused instances of `routine` get release at once, and it is impossible to release all instances of `routine` one at a time, sequentially. 
## 3.3. Types of values 
### 3.3.1. Number literals
A number literal is any string that may be parsed as a float. Unless specified to be strictly an integer, all numbers are parsed as double-precision floats (f64).  
It is important to make the distinction between number literals and numbers stored in items. While both are numbers, number literals are used more as specific values, while items represent containers for values in the actual program/level.  
It is also important to recognize that all floats in GD are 32 bits floats. This means that any integer values above 2^24, or 16 777 216, while correctly parsed by the compiler, may be incorrectly rounded by GD itself.
### 3.3.2. Item literals
An item literal represents a GD item, most commonly a counter or timer item. It is denoted as such:
- Counter: `CXXXX`, where `XXXX` represents the ID of the counter. Example: `C123` represents the counter with ID 123.
- Timer: `TXXXX`, where `XXXX` represents the ID of the timer. Example: `T456` represents the timer with ID 456.
IDs do not have to be 0-padded, and they must be in decimal form. They are only valid if they are in the range [1, 9999]. The same goes for IDs in group literals.   
Item literals are parsed by first checking for a prefix of either `C` or `T`, and if this is true, the rest of the literal is parsed as a base-10 signed 16-bit integer, since IDs are internally represented as signed 16-bit integers by GD.
### 3.3.3. Groups
Both of the following are interally groups:
#### 3.3.3.1. Group literals
Group literals refer to a static group ID. They are written as `g{id}`, where ID is a valid group ID.  
Group literals are parsed the same way as item literals, except for the prefix.  
Example: `g123` refers to the group with ID 123.
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
### 3.3.4. Aliases
Aliases act as substitutions for other values, namely, other items. They are used primarily to reference items that may not have a constant value.

<!-- Version number -->
As of TASM v0.2.2, the aliases that exist are:
- `MEMREG`: the [MEMREG](#124-memreg). Has a default value of `C9998`/`T9998`, but may change according to compiler arguments.
- `PTRPOS`: counter that stores the current pointer position (0-indexed).
- `MEMSIZE`: integer that stores the size of the memory. 0 if no memory exists.
- `ATTEMPTS`: refers to the number of attempts. This is a built-in item in GD.
- `POINTS`: refers to the points counter. This is a built-in item in GD.
- `MAINTIME`: refers to the MainTime timer. This is a built-in item in GD.
### 3.3.5. Strings
If a value was not parsed as any of the above, it is left as a string. Strings are rarely used in the language, but a notable use is as a label for an IOBlock.  
**Note: Since strings are the fallback, values that maybe be interpreted as another type are NOT parsed as strings. Please be midful of this when trying to pass a string argument which may, for example, also be a routine name, and thus will get parsed as a Group.**
### 3.3.6. Argsets 
Instructions may have different uses depending on the provided arguments. For this reason, they are explicitly typed. 
Since instruction arguments are typed, these types are checked during compilation in the [instruction parsing stage](#53-instruction-parsing). 
## 3.4. Group usage 
Group usage in TASM is meant to be optimized, but is not expected to be fully optimized while the language is still in development.   
Each routine uses one group to hold all of its instructions. After that, any instructions that need extra groups may use them. 
Below is the specification for all instructions and how many extra groups are used.

| Instruction                    | Groups      | Usage                                                                                  |
| ------------------------------ | ----------- | -------------------------------------------------------------------------------------- |
| Any arithmetic + MOV           | 0           | none                                                                                   |
| Spawn compare                  | 1           | Spawn trigger for group                                                                |
| Fork compare                   | 2           | Spawn triggers for both groups                                                         |
| SPAWN                          | 0           | none                                                                                   |
| Non-memory initializer         | 0           | none                                                                                   |
| NOP                            | 0           | none                                                                                   |
| Non-initializer memory command | 0           | none                                                                                   |
| MALLOC/FMALLOC                 | memsize + 4 | one for the pointer, pointer reset, read and write groups, and one per allocated cell. |
## 3.5. Comments
<!-- Version Number -->
A comment is anything that follows a semicolon (`;`) on the same line. Multi-line comments are not supported as of TASM v0.2.2. 
## 3.6. Execution model
The execution model of TASM is one fairly similar to that of real hardware:
- All instructions take some amount of time to execute, always an integer amount of ticks.
- Each group is assigned a primary group to start, though more are used per comparison instruction.
- Instructions are executed sequentially, and are placed from left to right when compiled to a level.
- Routines are always spawned with spawn-ordered enabled.
- Spawned routines execute concurrently, no matter how many of them there are.
# 4. TASM Toolkit
**NOTE:** As of April 12, 2026, there are no installers. The TASM compiler is entirely portable, and should be treated as such. 
It is encouraged to use the pre-built executables from the [GitHub repository](https://github.com/ArrowSlashArrow/tasm-lang), however, if it is not possible to use them, refer to the below instructions for manually installing the compiler:
## 4.1. rtasm compiler
Prerequisites: 
- Rust version v1.85.0 or later
Run `cargo build --release` to compile the executable. 
Navigate to the executable's directory by running `cd target/release/`, and to compile a TASM program, run `tasm.exe <program>.tasm`. 
## 4.2. pytasm compiler
**NOTE:** pytasm is currently deprecated, and will NOT receive future updates. It is *HIGHLY* recommended to use the rust compiler instead. 
**WARNING**: pytasm will **OVERWRITE** the first level in your savefile. Please be mindful of this when compiling a program. 

Prerequisites: 
- Python 3.9 
- All packages in requirements.txt installed 
	- If not installed, run `pip install -r requirements`.

Navigate to the `pytasm/` directory, and run `python main.py <program>.tasm` to compile the program. 
To see options, run `python main.py --help`. 
## 4.3. The interpreter/emulator 
Note: The interpreter is currently only accessible through the pytasm compiler

The interpreter is a powerful tool whose primary function is to emulate the GD environment, which is useful when trying to debug some program without compiling it every time. It should not be considered a 1:1 replica of the GD editor, as it does have a few minor quirks associated with it. 
To access the interpreter, first navigate to the `pytasm/` directory. Then, run `python main.py <program>.tasm --interpret`.
## 4.4. Getting started
It may be intimidating to use a language like this one, however, the language is intended to be easy to read and understand. While the language is verbose, it should not be considered unapproachable in any way.
## 4.4.1. Example programs
Example programs can be found in the `example_programs/` directory in the repo. Here are some of them:
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
    DISPLAY C1
    MALLOC 50
    INITMEM 0,1

fib:
    ; read the previous value
    MREAD
    MFUNC
    MOV C1, MEMREG ; read value from the memreg
    
    ; increment pointer and read the next number
    MPTR 1
    MFUNC

    ; add the previously stored value to the memreg, 
    ; to get the sum of the previous value and this one 
    ADD MEMREG, C1
    
    ; write the sum into the next memory cell
    MWRITE
    MPTR 1
    MFUNC
    
    ; move pointer back to the previous number in preparation for the next iteration
    MPTR -1

    SL fib, PTRPOS, 50
  
_start:
    SPAWN fib
```
This program generates the fibonacci sequence in the provided memory. The result memory reads as such: 0 1 1 2 3 5 8 13 ...  
This program uses 3 groups: one for the `_start` routine, one for the `fib` iteration routine, and one for the condition check at the end of the `fib` routine. It notably does not use a group for the `_init` routine, since all initializer functions correspond to structures instead of triggers.
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
This program checks whether the input value in C1 is prime. If so, it returns 1 in C5, otherwise it returns 2. It uses a total of 8 groups: 5 for routines, and 3 for comparisons.

# 5. Compiler spec 
This section is intended for advanced users and/or contributors. It is not necessary to read to use TASM.  
Note: this section is an overview of the compiler, and omits some details. To resolve any ambiguity, please read the compiler source code comments.  
The compiler executes the following sections in order:
## 5.1. Preprocessing 
Before anything other processing is done to the source code, some preprocessing is applied to it. 
The steps are as such: 
1. The source code is split into lines 
2. Each line is stripped of comments and whitespace on the right, and given an index 
3. All blank lines are removed 
4. All remaining lines are collected into a list 
These steps are done to minimize any spacing and/or formatting errors, since this language is mostly formatting-insensitive. 
## 5.2. Routine indexing 
Before any instruction parsing, all routines are first indexed. This is important to resolve all routines before any are referenced in instructions, and possibly (incorrectly) determined to be invalid. 
Routines are parsed as such: 
1. For each line, 
	1. if the line is not indented and ends with a colon (`:`), it is considered a routine identifier. The current routine identifier is set to this identifier (but without the ending colon)
	2. if the line is indented, it will be collected into a list of instructions associated with this routine
2. Routines are collected into a list of tuples: (routine starting line, routine identifier, routine group, routine lines with line numbers)
## 5.3. Instruction parsing 
If an instruction line is empty, it is skipped. Otherwise,
1. The instruction arguments are parsed like so:
	1. The first space character is found, and anything to the left of it is considered the instruction identifier, and anything to the right of the argset.
	2. The argset is split along each comma, and each argument is stripped of spaces on either side.
	3. Each argument is parsed as a TasmValue, which may be one of the types listed [earlier](#33-types-of-values).
2. Next, the matching identifier's instruction sets and their handlers, and whether this is an initializer instruction is pulled from the instruction spec table. 
3. check that this instruction is allowed in the routine if the routine is the initializer routine.
4. If the argset matches any set of types of that instruction, the respective argument handler function pointer and other relevant info (such as line number and type) is returned in an Instruction object. Otherwise, the parser throws an error.

## 5.4. Compilation to level
At this point, we have a complete set of routines with valid instructions, so the compiler assumes this.
Instructions are converted to objects in this manner:
1. Keep track of the current group, as well as the memory type and related information
2. For each routine,
	1. Determine the y-position of the group and reset object position
	2. Check that the current group does not exceed the group limit of 10,000. Throw an error and exit the compilation process if it does.
	3. For each instruction,
		1. Resolve aliases in the instruction argset
		2. Call the instruction handler function with the instruction's argset
		3. Add the returned object(s) to the level
		4. Update any data returned alongside the objects, which may include: extra groups used, amount of spaces to skip (on the x-axis), group of pointer collblock, etc.
		5. increment the x-position of the next object cluster by 1 + spaces to skip
3. If the group of the entry point is not 0, i.e. that the entry point either exists or has a group, add an IOBlock for it. 
## 5.5. Extended Backus-Naur grammar definition
Note: This grammar is **approximate**. It may allow some things that the compiler doesn't or overshadow details.
```
program ::= routine* ;

routine ::= identifier ":" newline instruction* ;

identifier ::= letter [string] ;

newline ::= "\n" | "\r\n" ;

instruction ::= argument { "," { " " } argument } ;

argument ::= string | number | alias | counter | timer ;

alias ::= "MEMREG" | "PTRPOS" ;

counter ::= "C" id ;

timer ::= "T" id ;

id ::= digit
	| digit digit
	| digit digit digit
	| digit digit digit digit ;

string ::= { letter | digit | "_" } ;

letter ::=  "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z" | "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" | "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" ;

number ::= ["-"] digit {digit} [ "." digit {digit} [ ("e" | "E") ["+" | "-"] digit {digit} ] ] ;

int ::= ["-"] digit {digit} ;
	
digit ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
```