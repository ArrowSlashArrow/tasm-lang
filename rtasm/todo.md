## todos for v1.0
- a way to include custom objects
    - `OBJECT`
- compiler optimizations (v1.0.0-rc1)
    - SORI (single object routine inlining)
    - optimizations within the compiler itself
- remove FMALLOC and have MALLOC allocate mixed memory, which mget/mset has an option to use floats for (flag or something)
- make landing page
- generate actual doc page from docs.md
- make either an installer or intsall mgr program (like rustup) for tasmc
- compiler optimizations
    - single object routine inline
        - any routine that conatins a single object will be inlined

## post-v1.0
- un-deprecate emulator
- possibly add tty for console output

## planned for PLSE
Note: instructions specified here are not planned to be included in the tasm ISA. they are placeholders for functionality in the planned stdlib for PLSE.

- boolean logic gates: `std::bool`
    - all instructions here work under the pretense that the operand(s) are strictly booleans.
    - AND, NAND, NOR, OR, XOR, XNOR
    - spawns group(s) based on condition
        - in the case of AND, if `a & b`, then group 1 is spawned
        - `group, counter, counter`: spawn
        - `group, group, counter, counter`: fork
        - `counter, counter [counter]`: assignment (value is computed and stored into result without any group spawning)
- branchless item compares that return booleans: `std::bool`
    - `==`: floor ( 0.5 / |a-b| + 0.5 )
    - `!=`: ceil ( |a-b| / |a-b| + 0.5 )
    - `>=`: floor ( a-b / (|a-b| + 0.5) + 1)
    - `>` : floor ( a-b / (|a-b| + 0.5) + 0.5)
    - `<=`: floor ( b-a / (|a-b| + 0.5) + 1)
    - `<` : floor ( b-a / (|a-b| + 0.5) + 0.5)
- `MODZ counter, counter, counter`: c1 = c2 % c3 == 0 (bool)
    - add spawn/fork variants to the above to immediately spawn groups (`SMODZ`, `FMODZ`)
    - a mod b == 0: 1 - ceil ( a/b - flr (a/b) )

- misc utils: `std::core`
    - `MAX counter, counter, counter`: c1 = max(c2, c3), same for min
        - max: ( a + b + |a - b| )/ 2
        - min: ( a + b - |a - b| )/ 2
    - `SWAP item, item`: swaps values

