## general
- add spawn delay + remap support to asm
- various other comp triggers 
- do not skip unindented strings that are not routine identifiers
- add flag args: `INSTR <args> | <flags>`
- InitRoutineMemoryAccess error
    - not allowed to run memory instructions in _init routine
- MultipleRoutineDefinitions error
    - not allowed to define different routines with the same names
- PointerOutsideMemory error
    - pointer cannot move more than MEMSIZE amount of spaces in one instruction. if it did, it's outside the memory block.
- fix disagreement between lexer-assigned group map and Tasm-assigned groups
- add ability to move pointer a dynamic amount with binary splitting
- add concurrent instruction prefix (`~`)
- stop using spawn ordered
   - use busy wait for sleep

## commands
### SPAWN command and derivatives

`SPAWN group1 + <float> @ <remaps>`
float: delay
remaps: remap dict. e.g. 192:168,0:11
`+` before `@` always

### `TSPAWN`
Args: `TSPAWN <timer>, <float>, <routine>`
Starts the timer specified, and when it counts up to the specified time (the second argument), the given routine is called.
internally uses timer trigger

###  `TSTART`
Args: `TSTART <timer>`
Starts this timer.
internally uses time ctrl trigger
### `TSTOP`
Args `TSTOP <timer>`
stops this timer.
internally uses time ctrl trigger

### `SRAND` / `FRAND`
Args: `SRAND <routine>, <float>`
Args: `FRAND <routine>, <routine>, <float>`
Spawns/Forks routines based on the chance
internally uses random trigger

### `RET`
Args: None
Returns from this routine.
internally uses stop trigger with ctrl id

all spawn triggers have a ctrl id that is the same as the group they're spawning
return: stop trigger that stops all objects with that control id (all spawn triggers that activate that group, and by proxy, the group itself)

### `WAIT`
Args: `WAIT <int>`

Waits for the given amount of ticks.

### `INSTRM` / `INSTRD`
Arithmetic instruction, except the result is multiplied/divided by the last argument.
This instruction is 1-tick.
The sum is computed, and then multiplied by the multiplier.
Arguments: `ADDM <item>, <item>, <number>`, `ADDM <item>, <item>, <item>, <number>`

this could potentially be in stored as a flag

### memory markers
marker objects that are in the memory structure.  
could help with moving a pointer to a previous location:
```
MOVEMARKER 1 ; move marker 1 to current location of pointer
; essentially store the current location of the pointer in the marker

MRESET
MPTR 50 ; goto some memory address

MREAD
MFUNC ; read it

MPTR M1 ; move pointer back to marker
```
the block at mem pos 0 can also be considered a marker  

### arithmetic
support some way to assign with an operator to items: `+=`, `/=`, etc.

this could potentially be in stored as a flag

### flags
a.k.a. "extra args"/ "extras"  
Very rough concept for passing extra arguments to instructions expandably, specifically to avoid many similar instructions.  
Consider the tasm where each variant has its own spearate instruction:
```
; base add
; C1 += C2
ADD     C1, C2
; add with multiplier
; C1 += C2 * 0.5
ADDM    C1, C2, 0.5
; add with divider
; C1 += C2 / 2
ADDD    C1, C2, 2
; add with multiplier, adding result to result item
; C3 += (C1 + C2) * 0.5
ADDMA   C3, C1, C2, 0.5
; add with divider, subtracting rounded result from result item 
; C3 -= round( (C1 + C2) / 2 )
RADDDS  C3, C1, C2, 2
; add with multiplier, dividing negated rounded result by result item, which is finally floored and made absolute
; C3 = | floor( C3 / -round( (C1 + C2) * 0.5 ) ) |
RNADDMSAF   C3, C1, C2, 0.5  ; preposterous!
...
```
vs. a tasm with flags to specify all possible configurations:
```
; base add
; C1 += C2
ADD     C1, C2
; add with multiplier
; C1 += C2 * 0.5
ADDM    C1, C2, 0.5
; add with divider
; C1 += C2 / 2
ADDD    C1, C2, 2
; add with multiplier, adding result to result item
; C3 += (C1 + C2) * 0.5
ADDM    C3, C1, C2, 0.5 | +=
; add with divider, subtracting rounded result from result item 
; C3 -= round( (C1 + C2) / 2 )
ADDD    C3, C1, C2, 2   | -= res:r
; add with multiplier, dividing negated rounded result by result item, which is finally floored and made absolute
; C3 = | floor( C3 / -round( (C1 + C2) * 0.5 ) ) |
ADDM    C3, C1, C2, 0.5 | /= res:r- fin:f+  ; a bit cleaner
...
```

Creating an instruction for each possible combinations would result in 5760 instructions total, which is simply unsistainable.  
While the flag system is arguably better for this situation, it still needs some work. For example, `res:r-` could be optionally written as `result:round-` or `res:-round` for disambiguation purposes. 

### Concurrent instructions
Concurrent instructions are isntructions that will be placed on the same x-position,
so that they will be executed on the same tick with spawn ordered.
Concurrent instructions should be denoted with `~`:

```
sequential:
    MOV C1, 1
    MOV C2, 2
    MOV C3, 3
    MOV C4, 4
    MOV C5, 5
    MOV C6, 6

concurrent:
    MOV C1, 1
    ~MOV C2, 2  ; will happen on the same tick as instruction above
    ~MOV C3, 3
    ~MOV C4, 4
    ~MOV C5, 5
    ~MOV C6, 6

```