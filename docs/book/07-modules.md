---
label: 7. Modules and the stdlib
order: 83
---

# Chapter 7: Modules and the standard library
As programs grow, you will want to split them into files and reuse code between projects. This chapter covers TASM's module system, and then tours the standard library that ships in the repository's [`stdlib`](https://github.com/ArrowSlashArrow/tasm-lang/tree/master/stdlib) folder.

## 7.1. Importing a module
Any `.tasm` file can be used as a module. To use one, `IMPORT` it in `_init`, giving its path relative to the current file, without the `.tasm` extension:

```x86asm
_init:
    IMPORT shapes               ; shapes.tasm, in the same folder
    IMPORT ../stdlib/math       ; math.tasm, in a different folder
```

You then refer to the module's routines and aliases as `module::name`, where `module` is the file name, without any folders:

```x86asm
_start:
    SPAWN shapes::circle_area
    SPAWN math::modulo
```

When you compile the program, the imported routines are copied into your level, each with its own group. The compiler renames routines internally, so two modules can both have a routine called `helper` without clashing.

## 7.2. Writing a library
A library is just a `.tasm` file without a `_start` routine. Here is [`shapes.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/shapes.tasm):

```x86asm
; A small library of geometry helpers.
; Every routine reads its input from T1 and writes its result to T2.
_init:
    import ../stdlib/consts

; T2 = area of a circle with radius T1 (pi * r^2)
circle_area:
    mul T2, T1, T1
    mul T2, consts::PI

; T2 = circumference of a circle with radius T1 (2 * pi * r)
circumference:
    mul T2, T1, consts::TAU
```

And here is [`use_shapes.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/use_shapes.tasm), which uses it:

```x86asm
; Uses the `shapes` library from this folder.
_init:
    import shapes

    display T1  ; radius
    display T2  ; latest result
    display T3  ; saved area

_start:
    mov T1, 5
    spawn shapes::circle_area
    wait 4                  ; give circle_area time to finish
    mov T3, T2              ; save the area

    spawn shapes::circumference
```

A few things to notice:
- `shapes` imports the standard library's `consts` module itself. Modules can import other modules, and the compiler resolves the whole chain. If two modules import the same module, it is only included once. Two modules that import each other are an error (E0045).
- When a library is imported, only the `ALIAS` and `IMPORT` instructions in its `_init` take effect. Anything else in a library's `_init`, such as `DISPLAY`, is ignored.
- To check a library for errors on its own, compile it with `--no-entry-point --no-export`, since it has no `_start`.

## 7.3. Calling conventions
TASM routines have no parameters or return values. Instead, the caller and the routine **agree** on which items to use. `shapes` documents its convention at the top of the file: input in `T1`, output in `T2`.

The standard library follows a similar convention:
- Inputs go in `C1`, `C2`, and so on (or `T1`, `T2`, ... for decimals).
- The result usually comes back in `C3`.
- Each routine documents its inputs and outputs in a comment above it, with `@param` and `@returns`.

There are also two ways to know when a routine has finished, since spawning does not wait:
1. **Wait for it.** A spawn ordered routine with *n* instructions finishes about *n* + 1 ticks after it is spawned. `use_shapes.tasm` waits 4 ticks for the two-instruction `circle_area`, which leaves some margin.
2. **Have it spawn the next step.** Some stdlib routines take the routine to continue with as an argument, as you will see in section 7.5.

Some stdlib routines are marked `@0tick`. These are designed to be spawned with `ordered:false`, so that they run in the same tick that they are spawned in.

## 7.4. A tour of the standard library

| Module | What it contains |
| ------ | ---------------- |
| `consts` | Useful constants as aliases: `PI`, `TAU`, `E`, `SQRT_2`, `INT_LIMIT`, `MAX_SAFE_INT` and more. |
| `math` | `modulo` (C1 mod C2 into C3), `sign` (the sign of C1 into C2), and more. |
| `compare` | Branchless comparisons, which return a 0/1 result in C3 instead of spawning a routine: `greater`, `less`, `less_eq`. |
| `bits` | Bitwise operations on 0/1 values: `and`, `or`, `not`, `nand`, `xor`, `nor`, `xnor`, and slow bit shifts. |
| `core` | Shared helpers and temporary items (`core::c1_temp`, ...) for other modules, and `wait_until`, which waits for a signal before continuing. |
| `debug` | `assert_eq_int` and `assert_eq_float`, which flash the background green if C1 equals C2 (or T1 equals T2), and red if not. |
| `mem_8bit` | A memory controller for reading and writing items 1 to 256 by address. See Chapter 8. |
| `mem_14bit` | A larger memory controller for addresses up to 16,384. |
| `vec` | A dynamically sized array. This module is still a work in progress and is not verified to work. |

The library files are short and well commented, and reading them is one of the best ways to learn idiomatic TASM.

### A worked example: modulo with a self-check
[`stdlib_modulo.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/stdlib_modulo.tasm) uses two stdlib modules:

```x86asm
; Uses the standard library to compute 47 mod 5, then checks the answer.
; The background flashes green if the answer is right, and red if it is wrong.
_init:
    import ../stdlib/math
    import ../stdlib/debug

    display C1
    display C2
    display C3

_start:
    ; math::modulo takes its inputs in C1 and C2 and returns C1 mod C2 in C3
    mov C1, 47
    mov C2, 5
    spawn math::modulo
    wait 4

    ; debug::assert_eq_int compares C1 with C2
    mov C1, C3
    mov C2, 2
    spawn debug::assert_eq_int
```

The program sets up the inputs, spawns `math::modulo`, and waits for it to finish. It then moves the result into the inputs of `debug::assert_eq_int`, along with the expected answer. Self-checks like this are a quick way to test your own library routines.

## 7.5. Passing arguments with remaps
Fixed items like `C1` and `C2` work for simple routines, but sometimes you need a routine to work on different items. GD's Spawn trigger can **remap** IDs: while the spawned routine runs, every use of one ID is replaced with another. In TASM, this is the `remap` flag of `SPAWN`:

```x86asm
    SPAWN bits::not | remap: {1 = 3}
```

`bits::not` normally reads `C1` and writes `C3`. With `1` remapped to `3`, it reads `C3` instead. The `bits` module uses this to build `nand` out of `and` and `not`:

```x86asm
; returns (c3): !(c1 & c2)
nand:
    spawn and       | ordered:false
    spawn not       | ordered:false remap: {1 = 3}
```

`and` leaves its result in `C3`, and the remapped `not` then inverts `C3` in place.

Remaps can also pass **routines** as arguments. The memory controller in the next chapter is spawned with its last step remapped to the operation that you want:

```x86asm
    spawn mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
```

Here, the group of `mem_8bit::mem_end` (an empty placeholder routine) is replaced with the group of `mem_8bit::cread`, so the controller finishes by running `cread`.

> [!WARNING]
> A remap applies to **every kind of ID** in the spawned routine, not just one. `remap: {1 = 3}` redirects counter 1, but also timer 1, group 1, and anything else with ID 1. Pick IDs for arguments that the routine does not use for anything else. This is why `vec.tasm` passes its arguments through high IDs such as `C9991`.

Remaps follow the same rules as other flags, with one addition: if a key or value contains `::`, you **must** put a space after `remap:`, as in `remap: {mem_8bit::mem_end = mem_8bit::cread}`. Without the space, the compiler mistakes the extra colons for the end of the flag name (E0030). It is a good habit to always write the space.

## 7.6. IMPORTSTD
TASM also has an `IMPORTSTD` instruction, which imports a module from the standard library folder of your TASM installation instead of from a relative path:

```x86asm
_init:
    IMPORTSTD math
```

> [!WARNING]
> In v0.3.2, `IMPORTSTD` is not yet practical to use: the standard library is not included in the release downloads, and the compiler looks for it in `stdlib/stdlib/` inside the installation folder. Until this is fixed, copy the `stdlib` folder next to your project and use `IMPORT` with a relative path, as all of the examples in this book do.

## Summary
- `IMPORT path` makes a module's routines and aliases available as `module::name`.
- A library is a `.tasm` file without `_start`. Only its aliases and imports are used from its `_init`.
- Routines communicate through agreed-upon items. Wait for a routine, or have it spawn the next step.
- The standard library has constants, math, comparisons, bitwise operations, debugging helpers and memory.
- `remap` lets you redirect a routine's items and groups, but it affects every kind of ID.

## Exercises
1. Add a `sphere_volume` routine to `shapes.tasm` (`4/3 * pi * r^3`), and call it from `use_shapes.tasm`.
2. Use `debug::assert_eq_float` to check that `shapes::circle_area` returns about 78.54 for a radius of 5. Does an exact comparison work for decimals?
3. Change `stdlib_modulo.tasm` so that it uses `remap` to make `math::modulo` read its inputs from `C10` and `C11` instead of `C1` and `C2`. Would the same remap be safe for a routine that spawns other routines? (Hint: which groups does your program use?)
4. Read `stdlib/bits.tasm`. How does `or` work in a single instruction?

Next: [Chapter 8: Dynamic memory](08-memory.md).
