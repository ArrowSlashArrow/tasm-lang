# .tasm
A computational language that compiles to Geometry Dash trigger objects.  
Not to be confused with the [Borland Turbo Assembler](https://en.wikipedia.org/wiki/Turbo_Assembler).  
  
tasm is currently in **version v0.1.0**.  
The basic instruction set is defined [here](docs.md), and the working compiler is in the root directory of the repo. Note that the compiler is not a standalone executable, and must be executed from source.

# Important information
**By default, the compiler OVERWRITES the first level in your savefile.** I have not implemented the patch for this yet, however PLEASE either make a temporary level when compiling or back up your savefile. 

I am currently working on a rust rewrite and polish of this program. For now, the python verion is still available, however it will soon be deprecated. Expect the rewrite to be complete and usable by Feb 15th, 2026.
  
# Guide
TASM (Trigger Assembly) is an assembly-like language that is made to simplify the process of working with Geometry Dash's mathematical operators. It is designed to take advantage of the new item edit and compare triggers that were added in version 2.2 of Geometry Dash. This toolkit features a documentation, a debugger, and a serialised to convert instructions to triggers that you can use in a level.  
  
Features:
* Turing-complete instruction set
* Optimised trigger placement and group usage
* Built-in memory system
* Compiles directly to trigger objects
<!-- * Versatile compiler, lots of options. -->
  
To create a program, first make a `.tasm` file.
In the new file, create two subroutines:
```
_init:
    ; init goes here
  
_start:
    ; code goes here
```
  
The `_init` subroutine will run before you play the GD level. This routine contains instructions such as `MALLOC` to allocate memory, or `DISPLAY` to display a counter.
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
    ADD C3, C1, C2 ; add c1 and c2, store the result in c3
  
_start:
    MOV C1, 2 ; set c1 to 2
    MOV C2, 3 ; set c2 to 3
    SPAWN add ; spawn the add subroutine
```
  
To compile the program, run `python main.py <file>` where `<file>` is the name of your program file. This will **OVERWRITE** the first level in your new levels if you do not add the `--append` option. Alternatively, you can make a new temporary level.
All of the options are displayed by appending `-h` to the command.
  
The resulting level should have the name of the program file, and is by default at the top.
  
# Notes 
* Counters are 32-bit signed ints. You can assign them to any number with item edit triggers, however the input values in those are stored as 32-bit floats, so larger numbers (specifically above 2^24) are incorrectly rounded due to floating point imprecision. Counters store values higher than 999,999,999 even if they display 999,999,999.
* To assign counters to really high numbers, use bit packing: Assign it the greater 16 bits, multiply by 65536, assign it the lesser 16 bits. Example 1000 \* 65536 + 1 yields 65536001 every time with no mistakes. This takes 3 instructions instead of 1. **THE `MOV` COMMAND ALREADY DOES THIS IF YOU ARE MOVING A NUMBER GREATER THAN 1,048,576. you can disable this with the flag `--no-bit-packing`.**
* Timers are useful to store floats, however their maximum value is 9,999,999.0
* Each instruction has a time to execute, which is denoted by `n-tick`, where `n` is the amount of ticks the instruction takes. Note that not all instructions are 1-tick, which can create race conditions if not accounted for when running multiple groups in parallel.
* Instructions that are `_init` routine-exclusive do not have a execution time, 
* Aliases exist for special counters: MEMREG (memory register value) for c9998, PTRPOS (address of pointer) for c9999.
* The geometry dash "CPU" is very different to a normal one: it only processes a maximum of 120 instructions per second per group (active routine), however it can run as many groups in parallel as necessary.
* The limit for block IDs, counter IDs, and groups is 9999, and any objects with corresponding values higher than that of the limit are not functional.
* The interpreter is a powerful tool that can be used to test your code in a way that does not require the launch of Geomtery Dash to run a .tasm file. To use it, add the `--interpret` flag to the end of your run command. The interpreter requires a \_start routine.
* The compiler is designed to compile to GD only on Windows as of right now. This may be subject to change in the future.
* Feel free to reach out to me on discord: @arrowslasharrow to ask me any questions!

# Instructions
Documentation for all instructions can be found [here](docs.md)