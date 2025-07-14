# .tasm Documentation
Here lies the documentation for all of the tasm instructions.
  
# Guide
.tasm is an assembly-like language that is made to simplify the process of working with Geometry Dash's mathematical operators.
  
Features:
 - Asynchronous
 - Turing-complete
 - Fast resulting triggers
  
To create a program, first make a `.tasm` file.
In the new file, create two subroutines:
```
_init:
    ; init goes here
  
_start:
    ; code goes here
```
  
The `_init` subroutine will run before you play the GD level.
The `_start` subroutine is the start point, similar to a main function.
Any additional subroutines may be defined as such:
```
subroutine_name:
    ; todo: code
```
  
Actual code must be written as instructions (see documentation below).
Instructions must be written under a subroutine, to which they belong, and indented by four spaces.
Comments can be added by putting a semicolon followed by the comment.
  
Example program:
```
_init:
    DISPLAY C1 ; show the c1 counter
  
add:
    ADD C3, C1, C2 ; c3 = c1 + c2
  
_start:
    MOV C1, 2 ; set c1 to 2
    MOV C2, 3 ; add 1 to c1
    SPAWN add ; spawn the add subroutine
```
  
To compile the program, run `python main.py <file>` here `<file>` is the name of your program file. This will **OVERWRITE** the first level in your new levels if you do not add the `--append` option. Alternatively, you can make a new temporary level.
All of the options are displayed by appending `-h` to the command.
  
The resulting level should have the name of the program file, and is by default at the top.
  
# Notes
* Counters are 32-bit signed ints. You can assign them to any number with item edit triggers, however the input values in those are stored as 32-bit floats, so larger numbers (specifically above 2^24) are incorrectly rounded due to floating point imprecision. They store values higher than 999,999,999 even if they display 999,999,999.
* To assign counters to really high numbers, use bit packing: Assign it the greater 16 bits, multiply by 65536, assign it the lesser 16 bits. Example 1000 \* 65536 + 1 yields 65536001 every time with no mistakes. This takes 3 instructions instead of 1. **THE MOV COMMAND ALREADY DOES THIS IF YOU ARE MOVING A NUMBER GREATER THAN 1,048,576. you can disable this with the flag `--no-bit-packing`.**
* Timers are useful to store floats, however their maximum value is 9,999,999.0
* Aliases exist for special counters: MEMREG (memory register value) for c9998, PTRPOS (address of pointer) for c9999, MEMSIZE (memory size) for c9997.
* The geometry dash "CPU" is very different to a normal one: it only processes a maximum of 240 instruction per second per group, however it can run as many group in parallel as necessary.
* The limit for block IDs, counter IDs, and groups is 9999, and any objects with corresponding values higher than that of the limit are not functional.
* The interpreter is a powerful tool that can be used to test your code in a way that does not require the launch of Geomtery Dash. To use it, add the `--interpret` flag to the end of your run command. The interpreter requires a \_start routine.
* The compiler is designed to compile to GD only on Windows as of right now. This may be subject to change in the future.
* Feel free to reach out to me on discord: @arrowslasharrow to ask me any questions!

# Instructions

## Arithmetic
### ADD
Arguments: `ADD <item> <number>`, `ADD <item> <item>`, `ADD <item> <item> <item>`
Adds the second argument to the first argument.
If there are three arguments, the second and third are added instead.
The result is stored in the first argument.
  
### SUB
Arguments: `SUB <item> <number>`, `SUB <item> <item>`, `SUB <item> <item> <item>`
Subtracts the second argument from the first argument.
If there are three arguments, the third is subtracted from the second instead.
The result is stored in the first argument.
  
### MUL
Arguments: `MUL <item> <number>`, `MUL <item> <item>`, `MUL <item> <item> <item>`
Multiplies the second argument by the first argument.
If there are three arguments, the second and third are multiplied instead.
The result is stored in the first argument.
  
### DIV
Arguments: `DIV <item> <number>`, `DIV <item> <item>`, `DIV <item> <item> <item>`
Divides the second argument by the first argument.
If there are three arguments, the second is divided by the third instead.
The result is stored in the first argument.
  
## Compares
`SE`, `SNE`, `SL`, `SLE`, `SG`, `SGE` all accept: `<routine> <item> <number>`, `<routine> <item> <item>`
SE: Spawns routine if a == b
SNE: Spawns routine if a != b
SL: Spawns routine if a < b
SLE: Spawns routine if a <= b
SG: Spawns routine if a > b
SGE: Spawns routine if a >= b
Does not pause the current group.
  
`FE`, `FNE`, `FL`, `FLE`, `FG`, `FE` all accept: `<routine> <routine> <item> <number>`, `<routine> <routine> <item> <item>`
FE: Spawns first routine if a == b otherwise spawns the second routine.
FNE: Spawns first routine if a != b otherwise spawns the second routine.
FL: Spawns first routine if a < b otherwise spawns the second routine.
FLE: Spawns first routine if a <= b otherwise spawns the second routine.
FG: Spawns first routine if a > b otherwise spawns the second routine.
FGE: Spawns first routine if a >= b otherwise spawns the second routine.
Does not pause the current group.
  
## Memory
### INITMEM
Arguments: `MALLOC <numbers>`
Assigns the numbers to memory in order, starting at address 0. Must be done after MALLOC. Numbers must for separated by commas, with no spacing.
Only allowed `_init` routine.
  
### MALLOC
Arguments: `MALLOC <number>`
Allocates a specified amount of counters to memory. Uses 1 group per counter + 4 groups.
Only allowed `_init` routine.
  
### MFUNC
Arguments: `MFUNC`
If the current memory mode is set to READ, then the value of the current memory location will be read to c9999.
If the current memory mode is set to WRITE, then the value of c9999 will be written to the current memory location.
  
### MREAD
Arguments: `MREAD`
Sets the memory mode to READ
  
### MWRITE
Arguments: `MWRITE`
Sets the memory mode to WRITE
  
### MPTR
Arguments: `MPTR <int>`
Moves the poiner by a specified amount.
Note: there is padding to prevent overflow/underflow, however if you move the pointer by a ridiculous amount, you will just not be able to read/write.
  
### MRESET
Arguments: `MRESET`
Resets the pointer position to 0
  
### MOV
Arguments: `MOV <item> <number>`, `MOV <item> <item>`
Copies the value of the second argument to the first argument.
  
## I/O
### IOBLOCK
Arguments: `IOBLOCK <group>`,
Places a block with a touchable spawn trigger to the specified group at the bottom.
Only allowed in the `_init` routine.
  
## Miscellaneous
### NOP
Arguments: `NOP`
Does nothing.
  
### SPAWN
Arguments: `SPAWN <routine>`
Spawns the corresponding routine.
Does not pause the current group.
  
### PERS
Arguments: `PERS <item>`
Makes the corresponding item persistent.
Only allowed in the `_init` routine.
  
### DISPLAY
Arguments: `DISPLAY <item>`
Adds a counter object for the corresponding item.
Only allowed in the `_init` routine.
  