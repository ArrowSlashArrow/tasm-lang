## general
- add style guidelines to docs
- refactor error enum with proper formatting via struct fields
    - add warning level
        - warning: modifying ptrpos. this counter should never be modified unless by actually moving the pointer.
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