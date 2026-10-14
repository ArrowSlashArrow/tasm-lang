---
label: 7. Modules and the stdlib
order: 83
---

# Chapter 7: Modules and the standard library
Once you have a few useful routines, you will want to use them in more than one program. In this chapter you will move routines into **modules**, import them, use the standard library, and use a large module, `bigint`, through its calling convention.

## 7.1. Your first module
A module is a `.tasm` file whose routines and aliases other programs can use. It doesn't need a `_start` routine. This one is based on [`example_programs/shapes.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/shapes.tasm), using the calling convention from [chapter 6](06-functions-and-remaps.md#64-a-calling-convention):

```nasm shapes.tasm
; Geometry helpers.
_init:
    importstd consts

    ; parameters
    alias radius,       t9901
    alias result,       t9902

; result = area of a circle (pi * radius²)
; @param radius
; @returns result
; @0tick
circle_area:
    mul result, radius, radius
    mul result, consts::PI

; result = circumference of a circle (2 * pi * radius)
; @param radius
; @returns result
; @0tick
circumference:
    mul result, radius, consts::TAU
```

To check a module for errors, compile it with `--no-entry-point`, since it has no `_start`:

```bash
tasmc shapes.tasm --no-entry-point --no-export
```

A program in the same folder imports it with `import`, and refers to its routines and aliases as `shapes::name`:

```nasm use_shapes.tasm
; Uses the `shapes` module from the same folder.
_init:
    import shapes

    alias radius,       t1
    alias area,         t2
    alias circle,       t3      ; circumference

    display radius
    display area
    display circle

_start:
    mov radius, 5
    spawn shapes::circle_area   | ordered:false delay:0 remap: {shapes::radius = radius, shapes::result = area}
    spawn shapes::circumference | ordered:false delay:0 remap: {shapes::radius = radius, shapes::result = circle}
```

The displays show `5`, `78.54` and `31.42`.

How imports work:
- `import path` takes a path relative to the importing file, without `.tasm`. `import lib/shapes` imports `lib/shapes.tasm`.
- The module is named after its file name: after `import lib/shapes`, its symbols are still `shapes::name`.
- `module::name` works for routines and aliases, in arguments and in remaps. Write a space after `remap:` when the dict contains `::`.
- Only `alias`, `import` and `importstd` in a module's `_init` take effect when it's imported. `display`, `ioblock`, `pers` and `event` in a module are ignored.
- The compiler only copies the routines that your program uses (and the ones they spawn) into the level.

Imports can be nested: `shapes` imports `consts`, and `use_shapes` doesn't need to know. Two modules that both import a third one are fine, but two modules can't import each other (E0045).

## 7.2. The standard library
`importstd` imports a module from the standard library, which is installed alongside the compiler. These modules are available:

| Module | Contains |
| ------ | -------- |
| `consts` | Constants: `PI`, `TAU`, `E`, `SQRT_2`, `LN_2`, `INT_LIMIT`, `MAX_SAFE_INT` and more. |
| `math` | Real-number functions on timers (`exp`, `ln`, `sqrt`, `pow`, `sin`, `atan2`, `tanh`, ...), integer functions on counters (`modulo`, `gcd`, `ipow`, `isqrt`, ...), and `min`/`max`/`clamp`/`lerp` for both. |
| `compare` | Branchless comparisons (`greater`, `less`, `less_eq`) and boolean gates (`and`, `or`, `not`, `xor`, ...) that write `1` or `0` to `c3`. |
| `debug` | Assertions that flash the background green or red. |
| `bits` | Bitwise operations on integers (`bitwise_and`, `bitwise_or`, `bitwise_xor`, `bitwise_not`) and bit shifts, with (`raw_shift_*`) or without (`fast_shift_*`) the sign bit. |
| `core` | Helpers for returning early and waiting for a signal. |
| `mem_8bit`, `mem_14bit` | Reading and writing items by address. See [chapter 9](09-memory.md). |
| `vec` | A list built on `mem_8bit`. Still in development. |

Each file in [`stdlib/`](https://github.com/ArrowSlashArrow/tasm-lang/tree/master/stdlib) documents its routines in comments. Read them before using a routine: several stdlib routines use low items such as `c1` to `c4` for their arguments, so they overwrite whatever your program keeps there.

> [!NOTE]
> `importstd` needs the standard library in the installation folder (`%APPDATA%\tasm\stdlib` or `~/.local/bin/tasm/stdlib`). The install scripts put it there for releases after v0.3.2. With older releases, copy the `stdlib/` folder from the repository there yourself, or use `import` with a relative path such as `import ../stdlib/math`.

## 7.3. Example: testing with `debug`
This program computes 47 mod 5 with `math::modulo` and checks the result with `debug::assert_eq_int`, which flashes the background green if `c1` equals `c2` and red if it doesn't. It is based on [`example_programs/stdlib_modulo.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/stdlib_modulo.tasm):

```nasm stdlib_modulo.tasm
; Computes 47 mod 5 with the standard library, then checks the answer.
; The background flashes green if the answer is right, and red if it is wrong.
_init:
    importstd math
    importstd debug

    alias EXPECTED,     2

    display c1
    display c2
    display c3

_start:
    ; math::modulo reads c1 and c2 and writes c1 mod c2 to c3
    mov c1, 47
    mov c2, 5
    spawn math::modulo          | ordered:false delay:0

    ; debug::assert_eq_int compares c1 with c2
    mov c1, c3
    mov c2, EXPECTED
    spawn debug::assert_eq_int  | ordered:false delay:0
```

Because both calls are 0-ticked, `c3` holds the result as soon as `math::modulo` returns, so no wait is needed in between.

The `compare` module turns comparisons into numbers. The style guide's branchless `sign` uses it:

```nasm stdlib_sign.tasm
; sign = (0 < x) - (x < 0), with the branchless comparisons from `compare`
_init:
    importstd compare

    display c1
    display c2

_start:
    mov c1, -7
    spawn sign                  | ordered:false delay:0

; c2 = sign of c1. Uses c3 and c4.
; @0tick
sign:
    mov c2, 0                   ; compare against 0
    spawn compare::greater      | ordered:false delay:0     ; c3 = (c1 > c2), i.e. 0 < x
    mov c4, c3                  ; save `0 < x`
    spawn compare::less         | ordered:false delay:0     ; c3 = (c1 < c2), i.e. x < 0
    sub c2, c4, c3              ; c2 = (0 < x) - (x < 0)
```

This uses three groups. [Chapter 5](05-fast-code.md#53-branchless-logic) showed a one-group version that uses division by zero instead.

## 7.4. Example: big numbers
Counters can't hold numbers beyond about 2 billion. [`example_programs/bigint.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/bigint.tasm) stores a big number in four counters called **limbs**, each holding three decimal digits:

```text
x = x0 + x1 * 1000 + x2 * 1000² + x3 * 1000³
```

so 123,456,789,012 is stored as `x3 = 123`, `x2 = 456`, `x1 = 789` and `x0 = 12`. It offers `add`, `sub`, `mul` and `div`, all 0-tickable, with parameters `lhs0`–`lhs3`, `rhs0`–`rhs3`, `res0`–`res3`, `rem0`–`rem3` and `flag`.

Copy `bigint.tasm` next to this program:

```nasm big_sum.tasm
; Adds 123,456,789,012 and 111,111,111,111 with `bigint`.
; The result, 234,567,900,123, is shown from the top display down.
_init:
    import bigint

    alias x0,           c1
    alias x1,           c2
    alias x2,           c3
    alias x3,           c4
    alias y0,           c5
    alias y1,           c6
    alias y2,           c7
    alias y3,           c8
    alias sum0,         c9
    alias sum1,         c10
    alias sum2,         c11
    alias sum3,         c12
    alias overflow,     c13

    ; displays stack upwards, so the lowest limb goes first
    display sum0
    display sum1
    display sum2
    display sum3
    display overflow

_start:
    ; x = 123,456,789,012
    mov x0, 12
    mov x1, 789
    mov x2, 456
    mov x3, 123
    ; y = 111,111,111,111
    mov y0, 111
    mov y1, 111
    mov y2, 111
    mov y3, 111
    spawn bigint::add           | ordered:false delay:0 noremap:true remap: {bigint::lhs0 = x0, bigint::lhs1 = x1, bigint::lhs2 = x2, bigint::lhs3 = x3, bigint::rhs0 = y0, bigint::rhs1 = y1, bigint::rhs2 = y2, bigint::rhs3 = y3, bigint::res0 = sum0, bigint::res1 = sum1, bigint::res2 = sum2, bigint::res3 = sum3, bigint::flag = overflow}
```

The remap is long, but it is all the calling convention needs: every parameter is mapped onto one of the program's counters. `overflow` is 0, because the sum fits in four limbs.

Reading `bigint.tasm` is a good way to see the techniques from chapters 5 and 6 at scale: `div` unrolls a 40-step loop with remaps, and `__div_step` uses a branchless update instead of a fork.

## 7.5. Writing a good module
[CONTRIBUTING.md](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/CONTRIBUTING.md) sets these rules for the standard library. They work well for any module:
- **Document every routine**: what it reads (`@param`), what it writes (`@returns`), and whether it can be 0-ticked (`@0tick`).
- **No magic IDs**: give every item and group the module uses a name with `alias`.
- **Use high IDs for parameters**, so that remapping them can't hit the caller's groups.
- **Mark internal routines and items** with a `__` prefix, so that users know not to call them.
- **No `display`, `ioblock`, `pers` or `event`**: they are ignored when the module is imported.
- **No `_start`**, and check the module with `--no-entry-point`.

## 7.6. Exercises
1. Move `modulo` from [chapter 6](06-functions-and-remaps.md#65-example-clock-with-remaps) into a module called `arith.tasm`, and change the clock program to import it.
2. Add `sphere_volume` (`4/3 * pi * radius³`) to `shapes.tasm`.
3. Use `bigint::mul` to compute 999,999 × 999,999. The result is 999,998,000,001.

Solution 1
```nasm arith.tasm
; Integer helpers.
_init:
    ; parameters of `modulo`
    alias dividend,     c9901
    alias divisor,      c9902
    alias remainder,    c9903
    alias __quotient,   c9904

; remainder = dividend mod divisor
; @param dividend
; @param divisor
; @returns remainder
; @0tick
modulo:
    fldiv __quotient, dividend, divisor
    mul __quotient, divisor
    sub remainder, dividend, __quotient
```

```nasm clock_module.tasm
; Splits a number of seconds into hours, minutes and seconds.
_init:
    import arith

    alias total,        c1
    alias hours,        c2
    alias minutes,      c3
    alias seconds,      c4
    alias rest,         c5
    alias per_hour,     c6
    alias per_minute,   c7
    alias INPUT,        9876

    display hours
    display minutes
    display seconds

_start:
    mov total, INPUT
    mov per_hour, 3600
    mov per_minute, 60
    spawn split_time            | ordered:false delay:0

; @0tick
split_time:
    spawn arith::modulo         | ordered:false delay:0 remap: {arith::dividend = total, arith::divisor = per_hour, arith::remainder = rest}
    sub hours, total, rest
    div hours, per_hour
    spawn arith::modulo         | ordered:false delay:0 remap: {arith::dividend = rest, arith::divisor = per_minute, arith::remainder = seconds}
    sub minutes, rest, seconds
    div minutes, per_minute
```

Solution 2
Add this routine to `shapes.tasm`:

```nasm shapes_sphere.tasm
_init:
    importstd consts

    alias radius,       t9901
    alias result,       t9902

; result = volume of a sphere (4/3 * pi * radius³)
; @param radius
; @returns result
; @0tick
sphere_volume:
    mul result, radius, radius
    mul result, radius
    mul result, consts::PI
    mul result, 4
    div result, 3
```

Solution 3
```nasm big_product.tasm
; Multiplies 999,999 by itself with `bigint`.
_init:
    import bigint

    alias x0,           c1
    alias x1,           c2
    alias x2,           c3
    alias x3,           c4
    alias p0,           c5
    alias p1,           c6
    alias p2,           c7
    alias p3,           c8
    alias overflow,     c9

    display p0
    display p1
    display p2
    display p3

_start:
    mov x0, 999
    mov x1, 999
    mov x2, 0
    mov x3, 0
    ; 999,999 * 999,999 = 999,998,000,001: p3 = 999, p2 = 998, p1 = 0, p0 = 1
    spawn bigint::mul           | ordered:false delay:0 noremap:true remap: {bigint::lhs0 = x0, bigint::lhs1 = x1, bigint::lhs2 = x2, bigint::lhs3 = x3, bigint::rhs0 = x0, bigint::rhs1 = x1, bigint::rhs2 = x2, bigint::rhs3 = x3, bigint::res0 = p0, bigint::res1 = p1, bigint::res2 = p2, bigint::res3 = p3, bigint::flag = overflow}
```
Both operands are remapped onto the same counters, which `bigint` allows.

In the next chapter, your programs start interacting with the level: the player, its inputs, and objects in the level.
