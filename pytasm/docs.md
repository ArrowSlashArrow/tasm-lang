# Arithmetic
All arithmetic instructions are 1-tick.
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
Arguments: `MUL <item> <number>`, `MUL <item> <item>`, `MUL <item> <item> <item>`, `MUL <item> <item> <number>`  
Multiplies the second argument by the first argument.  
If there are three arguments, the second and third are multiplied instead.  
The result is stored in the first argument.  
  
### DIV
Arguments: `DIV <item> <number>`, `DIV <item> <item>`, `DIV <item> <item> <item>`, `DIV <item> <item> <number>`  
Divides the second argument by the first argument.  
If there are three arguments, the second is divided by the third instead.  
The result is stored in the first argument.  

### FLDIV
Same as `DIV`, except the result is rounded down to the nearest integer.
  
# Compares
Spawning a group does not automatically pause the parent group.  
The spawned group will start execution 2 ticks after the compare instruction
is executed.  
All compare instructions are 2-tick.

`SE`, `SNE`, `SL`, `SLE`, `SG`, `SGE` all accept: `<routine> <item> <number>`, `<routine> <item> <item>`
* SE: Spawns routine if a == b
* SNE: Spawns routine if a != b
* SL: Spawns routine if a < b
* SLE: Spawns routine if a <= b
* SG: Spawns routine if a > b
* SGE: Spawns routine if a >= b
  
`FE`, `FNE`, `FL`, `FLE`, `FG`, `FE` all accept: `<routine> <routine> <item> <number>`, `<routine> <routine> <item> <item>`
* FE: Spawns first routine if a == b otherwise spawns the second routine.
* FNE: Spawns first routine if a != b otherwise spawns the second routine.
* FL: Spawns first routine if a < b otherwise spawns the second routine.
* FLE: Spawns first routine if a <= b otherwise spawns the second routine.
* FG: Spawns first routine if a > b otherwise spawns the second routine.
* FGE: Spawns first routine if a >= b otherwise spawns the second routine.
  
# Memory
### INITMEM
Arguments: `MALLOC <numbers>`
Assigns the numbers to memory in order, starting at address 0. Must be done after MALLOC. Numbers must for separated by commas, with no spacing.
Only allowed `_init` routine.
  
### MALLOC
Arguments: `MALLOC <positive int>`
Allocates a specified amount of counters to memory. Uses 1 group per counter + 4 groups.
Only allowed `_init` routine.
### FMALLOC
Arguments: `FMALLOC <positive int>`
Allocates a specified amount of timers (floats) to memory. Uses 1 group per timer + 4 groups.
Only allowed `_init` routine.
  
### MFUNC
Arguments: `MFUNC`
If the current memory mode is set to READ, then the value of the current memory location will be read to c9999.
If the current memory mode is set to WRITE, then the value of c9999 will be written to the current memory location.
Execution time: 2 ticks.  
  
### MREAD
Arguments: `MREAD`
Sets the memory mode to READ
Execution time: 1 tick.  
  
### MWRITE
Arguments: `MWRITE`
Sets the memory mode to WRITE
Execution time: 1 tick.  
  
### MPTR
Arguments: `MPTR <int>`
Moves the poiner by a specified amount.
Note: there is padding to prevent overflow/underflow, however if you move the pointer by a ridiculous amount, you will just not be able to read/write.
Execution time: 1 tick.  
  
### MRESET
Arguments: `MRESET`
Resets the pointer position to 0
Execution time: 1 tick.  
  
### MOV
Arguments: `MOV <item> <number>`, `MOV <item> <item>`
Copies the value of the second argument to the first argument.
Execution time: 1 tick.  
    
# Miscellaneous
### NOP
Arguments: `NOP`
Does nothing.
Execution time: 1 tick.  
  
### SPAWN
Arguments: `SPAWN <routine>`
Spawns the corresponding routine.
Does not pause the current group.
Execution time: 1 tick.  
  
### PERS
Arguments: `PERS <item>`
Makes the corresponding item persistent.
Only allowed in the `_init` routine.
  
### DISPLAY
Arguments: `DISPLAY <item>`
Adds a counter object for the corresponding item.
Only allowed in the `_init` routine.

### IOBLOCK
Arguments: `IOBLOCK <group>, <int>, <string>`,
Places a block at the bottom of the level, at the specified x-position (2nd argument) with an annotation (3rd argument).
Also places a touchable spawn trigger that spawns the specified group. 
Intended as a debug feature and/or substitute for user input.
Only allowed in the `_init` routine.

# Version
TASM instruction set version: **v0.1.0**