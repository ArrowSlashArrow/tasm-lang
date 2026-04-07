## general
- docs
    - add style guidelines/best practices
    - add ptrpos counter inc/dec docs to mptr/mreset instructions
- refactor error enum with proper formatting via struct fields
    - add warning level
        - warning: modifying ptrpos. this counter should never be modified unless by actually moving the pointer.
- `--no-log` flag to disable everything printed to stdout
- `--objdump` don't compile, but dump all object info once parsed

## roadmap
- 0.2.x: utility releases
    - implement boolean data operations (0.2.3)
        - boolean logic gates
            - all instructions here work under the pretense that the operand(s) are strictly booleans.
            - AND, NAND, NOR, OR, XOR, XNOR
            - spawns group(s) based on condition
                - in the case of AND, if `a & b`, then group 1 is spawned
                - `group, counter, counter`: spawn
                - `group, group, counter, counter`: fork
                - `counter, counter [counter]`: assignment (value is computed and stored into result without any group spawning)
        - branchless item compares that return booleans
            - `==`: floor ( 0.5 / |a-b| + 0.5 )
            - `!=`: ceil ( |a-b| / |a-b| + 0.5 )
            - `>=`: floor ( a-b / (|a-b| + 0.5) + 1)
            - `>` : floor ( a-b / (|a-b| + 0.5) + 0.5)
            - `<=`: floor ( b-a / (|a-b| + 0.5) + 1)
            - `<` : floor ( b-a / (|a-b| + 0.5) + 0.5)

    - more utils (0.2.2)
        - `MAX counter, counter, counter`: c1 = max(c2, c3), same for min
            - max: ( a + b + |a - b| )/ 2
            - min: ( a + b - |a - b| )/ 2
        - `CLAMP`, `STEP`
        - `MODZ counter, counter, counter`: c1 = c2 % c3 == 0 (bool)
            - add spawn/fork variants to the above to immediately spawn groups (`SMODZ`, `FMODZ`)
        - `SWAP item, item`: swaps values

- memory improvements: v0.3.0
    - this is a possibly breaking change, so minor release number is increased
    - refactor memory to be more group efficient
    - refactor should also include being able to look up memory from any address
    - possibly retain legacy memory as compiler option
    - mem instructions overhaul
        - `INITMEM <ints>`: keeping it
        - `(F)MALLOC <start>, <end>`: specify range instead of allocsize. allocsize is stored as `MEMSIZE` alias anyway, so it doesn't matter. removes need for `--mem-end-counter` flag.
        - `MGET`: gets value at PTRPOS and stores it in memreg.
        - `MSET`: sets value in memreg to PTRPOS.
        - `MRESET`: sets addr to 0.
        - `LMA <addr>`: load mem addr, shorthand for `MOV PTRPOS, <addr>`.
        - `MPTR`/`MREAD`/`MWRITE`/`MFUNC`: deprecated

- 0.3.x: optimizations update
    - compiler optimizations (v0.3.1)
        - SORI (single object routine inlining)
        - optimizations within the compiler itself
- 0.4.0
    - un-deprecate emulator

- 0.5.0
    - possibly add tty for console output

### memory markers (legacy memory)
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

### compiler optimizations
- single object routine inline
    - any routine that conatins a single object will be inlined

## extas, for later
- make landing page
- generate actual doc page from docs.md
- make either an installer or intsall mgr program (like rustup) for tasmc