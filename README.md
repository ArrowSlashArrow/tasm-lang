# .tasm
A computational language that compiles to Geometry Dash trigger objects.  
Not to be confused with the [Borland Turbo Assembler](https://en.wikipedia.org/wiki/Turbo_Assembler).  
  
tasm is currently in **version v0.1.0**.  
The working compiler is in the `rtasm/` directory. Note that the compiler is not a standalone executable, and must be executed from source.

Documentation may be found [here](docs.md).

# Project information
The working tasm compiler is located in the `rtasm` directory, and the deprecated python compiler is located in the `pytasm` directory. It is reccomended to use the `rtasm` compiler, since it is faster and more robust.  
The emulator, located in the `interpreter` directory, is not confirmed to be fully accurate, and should not be considered a 1:1 replica of GD's environment. That said, it is still a good tool for debugging tasm. 
  
# Quick start
TASM (Trigger Assembly) is an assembly-like language that is made to simplify the process of working with Geometry Dash's mathematical operators. It is designed to take advantage of the new item edit and compare triggers that were added in version 2.2 of Geometry Dash. This toolkit features a documentation, a debugger, and a serialised to convert instructions to triggers that you can use in a level.  
  
Features:
* Turing-complete instruction set
* Optimised trigger placement and group usage
* Built-in memory system
* Compiles quickly and directly to trigger objects
* Integration with dedicated backend: [GDlib](https://crates.io/crates/gdlib)
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
Instructions must be written under a subroutine, to which they belong, and indented by at least spaces.
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
  
To compile the program, navigate to `rtasm` and run `cargo run <file>` where `<file>` is the name of your program file. For the compiler to work, you must have rust installed. 
The resulting level should have the name of the program file, and is by default at the top.
  
## Notes 
* The compiler is designed to compile to GD only on Windows as of right now. This may be subject to change in the future.
* Feel free to reach out to me on discord: @arrowslasharrow to ask me any questions!