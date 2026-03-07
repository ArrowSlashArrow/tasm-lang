## general
- add spawn delay + remap support to asm
- add flag args: `INSTR <args> | <flags>`
- add ability to move pointer a dynamic amount with binary splitting
- aliases todo:
    - MEMSIZE
    - POINTS
    - ATTEMPTS
    - MAINTIME
- workflow for pr to run all tests
- define command (like #define in c)
    - defines a constant that can be used as an alias
    - cannot overwrite existing aliases (any already defined and any of the default ones)
    - e.g. `ALIAS external_object, g123`
        - `external_object` now refers to the group `123`
    - init only
- make the release mode toggle actually do something
    - currently it is ignored and everything is compiled in release anyways
    - debug (not release) mode:
        - comments are present alongside each routine in the form of text objects
    - release mode:
        - all labels except for "memory" and routine labels are removed
- add style guidelines to docs
- refactor error enum with proper formatting via struct fields

## commands
### `TSPAWN`
Args: `TSPAWN <timer>, <float>, <routine>`
Starts the timer specified, and when it counts up to the specified time (the second argument), the given routine is called.
internally uses timer trigger  

Planned for v0.1.2.

###  `TSTART`
Args: `TSTART <timer>`
Starts this timer.
internally uses time ctrl trigger
### `TSTOP`
Args `TSTOP <timer>`
stops this timer.
internally uses time ctrl trigger

### `RET`
Args: None
Returns from this routine.
internally uses stop trigger with ctrl id

all spawn triggers have a ctrl id that is the same as the group they're spawning
return: stop trigger that stops all objects with that control id (all spawn triggers that activate that group, and by proxy, the group itself)

Related instructions: 
* `PAUSE <routine>`: pauses the routine. unpausable via:
* `RESUME <routine>`: unpauses the routine.
* `STOP <routine>`: pauses and exits the routine. not resumable.

Control flow instructions require that the spawner object has a known control ID. 
This ID will be set to the group that it is responsible for calling. If it responsible for calling multiple groups, it should not be given any control ID. For example, random and andvanced random triggers will not be given a control ID. This is because each object ma
As a result, control flow instructions are not expected to work if the routine can be spawned by an advanced random trigger. Alterntaively, a manual control ID flag may be set for the random spawn instructions. This flag may contain anything that corresponds to a group: either a group literal or a routine identifier.  
Planned for v0.1.3.

### `INSTRM` / `INSTRD`
Arithmetic instruction, except the result is multiplied/divided by the last argument.
This instruction is 1-tick.
The sum is computed, and then multiplied by the multiplier.
Arguments: `ADDM <item>, <item>, <number>`, `ADDM <item>, <item>, <item>, <number>`

^ this will be a flag, mod:float
^^ ADDM and SUBM will be included as utility functions. if the mod flag on those is specified, it overrides the argument.

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

planned for v0.3.0

### arithmetic
support some way to assign with an operator to items: `+=`, `/=`, etc.

this will be a flag: {op}= (+=, -=, *=, /=)

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

for spawning: delay:float, remap:{group:group}

planned for v0.2.0

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

planned for v0.4.0

### compiler optimizations
- single object routine inline
    - any routine that conatins a single object will be inlined

## extas, for later
- make landing page
- generate actual doc page from docs.md
- make either an installer or intsall mgr program (like rustup) for tasmc