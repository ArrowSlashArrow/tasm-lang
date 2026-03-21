## general
- add style guidelines to docs
- refactor error enum with proper formatting via struct fields
    - add warning level
        - warning: modifying ptrpos. this counter should never be modified unless by actually moving the pointer.
- add logo to repo in assets/
- `--no-log` flag to disable everything printed to stdout
- `--objdump` don't compile, but dump all object info once parsed

## roadmap
- 0.2.x: utility releases
    - instruction flags (v0.2.0)
    - custom aliases (v0.2.1)
    - memory improvements (v0.2.2)
        - memory markers
        - dynamic movement of pointer via binary splitting
- 0.3.x: optimizations update
    - concurrent instructions (v0.3.0)
    - compiler optimizations (v0.3.1)
        - SORI (single object routine inlining)
        - optimizations within the compiler itself
- 0.4.0
    - un-deprecate emulator

### memory markers
marker objects that are in the memory structure.  
could help with moving a pointer to a previous location:
```
MVMARK 1 ; move marker 1 to current location of pointer
; essentially store the current location of the pointer in the marker

MRESET
MPTR 50 ; goto some memory address

MREAD
MFUNC ; read it

MPTR M1 ; move pointer back to marker
```
the block at mem pos 0 can also be considered a marker  

### `ALIAS` (init-only)
- defines a constant that can be used as an alias
- cannot overwrite existing aliases (any already defined and any of the default ones)
- e.g. `ALIAS external_object, g123`
    - `external_object` now refers to group 123
- alias is resolved only when mentioned

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

Flags must be formatted in a very specific way:
- They must all be after one and only one | in the instruction line
- All flag-value pairs must be separated by whitespace
- There must not be any space between the identifer, the colon separator, and its value
    - Dictionary values may contain spaces, but only between the comma and the next value.

#### planned flags
- `resmode`: specifies rounding and sign mode of result between ids
    - accepts a compound string of the following in any order:
        - a `-` or `+`
        - round argument (`r`/`round`, `c`/`ceil`, `f`/`floor`)
- `finmode`: specifies rounding and sign mode of final result
    - accepts a compound string of the following in any order:
        - a `-` or `+`
        - round argument (`r`/`round`, `c`/`ceil`, `f`/`floor`)
- `itemmod`: sets itemedit modifier
    - accepts a float which is the mod is set as.
    - overrides `ADDM`/`SUBM` mod if specified.
- `divmod`: divides id result by modulus instead of multiplying it
- `iter`: compound assignment operator. result is always assigned to unless this flag is specified. 
- `op`: operator between IDs.
    - accepts one of the following: `+`, `-`, `/`, `*`
- `delay`: specifies delay of spawn triggers of this command
    - accepts a float (amount of seconds) for delay.
    - delay variation will not be supported.
- `remap`: spawn remap of the spawn trigger. *only* for `SPAWN`.
    - accepts a dict in the format `{id:remap}`
    - e.g. `remap:{125:126, 200:300}` remaps 125 to 126 and 200 to 300
- `tpaused` : bool (starts timer paused)
- `tmod` : float (timemod for timer)
- `tstop` : pauses timer when the target time is reached
- `nover` : doesnt start timer according to this rule (docs in gdlib::gdobj::triggers::time_trigger) 

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

### compiler optimizations
- single object routine inline
    - any routine that conatins a single object will be inlined

## extas, for later
- make landing page
- generate actual doc page from docs.md
- make either an installer or intsall mgr program (like rustup) for tasmc