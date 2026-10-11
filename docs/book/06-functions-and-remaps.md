---
label: 6. Routines as functions
order: 84
---

# Chapter 6: Routines as functions
In most languages, a function takes arguments and returns a result. TASM routines read and write fixed items instead. In this chapter you will learn two ways to pass data to routines: copying values into fixed items, and **remaps**, which the style guide recommends. You will also see how to pass *routines* as arguments.

## 6.1. Arguments in fixed items
The simplest convention: a routine reads its inputs from certain items and writes its result to another. The caller copies its values in, spawns the routine 0-ticked, and copies the result out.

```nasm modulo_copy.tasm
_init:
    alias dividend,     c1      ; input of `modulo`
    alias divisor,      c2      ; input of `modulo`
    alias remainder,    c3      ; output of `modulo`
    alias quotient,     c4      ; used inside `modulo`
    alias a,            c10
    alias b,            c11
    alias a_mod_7,      c12
    alias b_mod_7,      c13

    display a_mod_7
    display b_mod_7

_start:
    mov a, 100
    mov b, 45

    mov dividend, a
    mov divisor, 7
    spawn modulo                | ordered:false delay:0
    mov a_mod_7, remainder      ; 2

    mov dividend, b
    spawn modulo                | ordered:false delay:0
    mov b_mod_7, remainder      ; 3

; remainder = dividend mod divisor
; @0tick
modulo:
    fldiv quotient, dividend, divisor
    mul quotient, divisor
    sub remainder, dividend, quotient
```

This works, and [`stdlib/math.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/stdlib/math.tasm) uses it: `math::modulo` reads `c1` and `c2` and writes `c3`. But every call costs extra instructions to copy values around, and the caller has to keep its own data out of `c1` to `c4`.

## 6.2. Remaps
A spawn can **remap** IDs: inside the spawned routine, one ID is replaced by another. With remaps, `modulo` can work directly on the caller's items:

```nasm modulo_remap.tasm
_init:
    alias dividend,     c1      ; input of `modulo`
    alias divisor,      c2      ; input of `modulo`
    alias remainder,    c3      ; output of `modulo`
    alias quotient,     c4      ; used inside `modulo`
    alias a,            c10
    alias b,            c11
    alias seven,        c12
    alias a_mod_7,      c13
    alias b_mod_7,      c14

    display a_mod_7
    display b_mod_7

_start:
    mov a, 100
    mov b, 45
    mov seven, 7
    spawn modulo                | ordered:false delay:0 remap: {dividend = a, divisor = seven, remainder = a_mod_7}
    spawn modulo                | ordered:false delay:0 remap: {dividend = b, divisor = seven, remainder = b_mod_7}

; remainder = dividend mod divisor
; @0tick
modulo:
    fldiv quotient, dividend, divisor
    mul quotient, divisor
    sub remainder, dividend, quotient
```

`remap: {dividend = a, ...}` means "inside `modulo`, use `a` wherever it uses `dividend`". Remaps work on IDs, so you can't remap an ID to a number; that's why 7 is stored in an item called `seven` first.

The syntax of `remap`:
- Pairs of `from = to` inside braces, separated by commas. Spaces are allowed.
- Each side can be a number, an item or group literal (`c5`, `g12`), a routine name, or an alias of one of these.
- Put a space between `remap:` and `{`. It is required when the dict contains `module::name` (chapter 7), so the style guide writes it everywhere.

## 6.3. Remaps change every kind of ID
> [!WARNING]
> A remap swaps **every kind of ID** with that number. `remap: {c1 = c10}` replaces counter 1 with counter 10 inside the routine, but also timer 1, group 1 and collision block 1.

In `modulo_remap.tasm`, `remap: {dividend = a}` is really `remap: {1 = 10}`. Group 1 is the group of the first routine, `_start`. `modulo` doesn't use any groups, so nothing goes wrong. But if `modulo` spawned `_start`, or any routine that it spawns did, the remap would redirect that spawn to group 10.

The style guide's advice is to keep parameters on IDs that nothing else uses. Routines get groups from 1 upwards, so high IDs are safe. [`bigint.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/bigint.tasm) uses 9901 to 9991, and [`stdlib/vec.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/stdlib/vec.tasm) uses 9991 to 9994. When in doubt, `tasmc -l` lists the group of every routine.

## 6.4. A calling convention
Putting this together gives the convention that `bigint.tasm` follows:
1. Each parameter is an alias for an item with a high ID.
2. Callers pass arguments by remapping the parameters onto their own items.
3. Items used only inside the routine also get high IDs, and their names start with `__` to mark them as internal.
4. A comment above the routine says what it reads, what it writes and whether it can be 0-ticked.

```nasm modulo_lib.tasm
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

A parameter that a caller doesn't remap falls back to the item it is an alias for. That makes good default values, and it means the result of a call can be read directly from `remainder` if it isn't remapped.

## 6.5. Example: clock with remaps
The clock from [chapter 2](02-items-and-arithmetic.md#27-example-clock) computed two remainders by hand. With `modulo`, it becomes:

```nasm clock_remap.tasm
; Splits a number of seconds into hours, minutes and seconds.
_init:
    alias total,        c1
    alias hours,        c2
    alias minutes,      c3
    alias seconds,      c4
    alias rest,         c5      ; seconds left after taking out the hours
    alias per_hour,     c6      ; 3600, as an item so that it can be remapped
    alias per_minute,   c7      ; 60
    alias INPUT,        9876    ; 2 h 44 min 36 s

    ; parameters of `modulo`
    alias dividend,     c9901
    alias divisor,      c9902
    alias remainder,    c9903
    alias __quotient,   c9904

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
    ; rest = total mod 3600, hours = (total - rest) / 3600
    spawn modulo                | ordered:false delay:0 remap: {dividend = total, divisor = per_hour, remainder = rest}
    sub hours, total, rest
    div hours, per_hour
    ; seconds = rest mod 60, minutes = (rest - seconds) / 60
    spawn modulo                | ordered:false delay:0 remap: {dividend = rest, divisor = per_minute, remainder = seconds}
    sub minutes, rest, seconds
    div minutes, per_minute

; remainder = dividend mod divisor
; @0tick
modulo:
    fldiv __quotient, dividend, divisor
    mul __quotient, divisor
    sub remainder, dividend, __quotient
```

`split_time` is 0-tickable because `modulo` is: the whole clock is computed in one tick.

## 6.6. Passing routines
Groups can be remapped too, which lets you pass a routine to another routine. The called routine spawns a **group parameter**, and the caller remaps that parameter to the routine it wants to run. [`stdlib/vec.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/stdlib/vec.tasm) does this to decide what happens after a bounds check.

This program counts how many numbers from 1 to 30 are multiples of 3 (fizz) and of 5 (buzz), using one routine that spawns `on_multiple` or `on_other` depending on the result:

```nasm fizz_buzz.tasm
; Counts multiples of 3 and of 5 from 1 to 30. The answers are 10 and 6.
_init:
    alias n,            c1
    alias three,        c2
    alias five,         c3
    alias fizz_count,   c4
    alias buzz_count,   c5
    alias LIMIT,        30

    ; parameters of `branch_on_multiple`
    alias dividend,     c9901
    alias divisor,      c9902
    alias on_multiple,  g9903   ; spawned if divisor divides dividend
    alias on_other,     g9904   ; spawned if it doesn't
    alias __multiple,   c9905

    display fizz_count
    display buzz_count

_start:
    mov n, 0
    mov three, 3
    mov five, 5
    spawn next_number           | ordered:false

; not 0-tickable: loops by spawning itself
next_number:
    add n, 1
    spawn branch_on_multiple    | ordered:false delay:0 remap: {dividend = n, divisor = three, on_multiple = fizz, on_other = nothing}
    spawn branch_on_multiple    | ordered:false delay:0 remap: {dividend = n, divisor = five, on_multiple = buzz, on_other = nothing}
    sl next_number, n, LIMIT    | ordered:false

; spawns `on_multiple` if divisor divides dividend, and `on_other` if not
; @0tick, as long as the routines it spawns are
branch_on_multiple:
    fldiv __multiple, dividend, divisor
    mul __multiple, divisor
    ife on_multiple, on_other, __multiple, dividend

fizz:
    add fizz_count, 1

buzz:
    add buzz_count, 1

nothing:
    nop
```

In the remap, `fizz`, `buzz` and `nothing` are routine names, which stand for their groups.

> [!NOTE]
> GD only lets a routine run once at a time *with the same remaps*. A spawn with different remaps counts as a different configuration (see [style guide, 2.3](../style-guide.md#23-a-routine-cant-0-tick-itself)).

## 6.7. Nested remaps
Remaps carry through nested spawns. If routine A is spawned with `remap: {c1 = c2}` and A spawns B with `remap: {c2 = c3}`, then B sees `c3` wherever it uses `c1`. This is easy to lose track of when one function calls another.

`noremap:true` turns on GD's **Reset Remap** option for a spawn, so the spawned routine only uses the remaps of that spawn and doesn't inherit any. `bigint.tasm` asks its callers to always use it:

```nasm
;     spawn bigint::add   | ordered:false delay:0 noremap:true remap: {bigint::lhs0 = x0, ...}
; - Use `noremap:true`, so that a call never depends on how GD combines nested remaps.
; - Any parameter that isn't remapped falls back to a counter that is internal to this module.
; - The result may be remapped onto the same counters as an operand (e.g. `x = x * y`).
; - The program must not use IDs 9901 to 9991 for its own groups or items, since remaps
;   swap every kind of ID.
```

Using `bigint` itself requires importing it, which is the topic of [chapter 7](07-modules-and-the-stdlib.md#74-example-big-numbers).

## 6.8. Exercises
1. Write an `average` routine with parameters `first`, `second` and `mean`, and use it to average two pairs of timers.
2. Write a `toggle` routine that switches an item between 0 and 1, and use it to make two lights blink at different speeds.

Solution 1
```nasm average.tasm
_init:
    alias a,            t1
    alias b,            t2
    alias ab_mean,      t3
    alias c,            t4
    alias d,            t5
    alias cd_mean,      t6

    ; parameters of `average`
    alias first,        t9901
    alias second,       t9902
    alias mean,         t9903

    display ab_mean
    display cd_mean

_start:
    mov a, 3
    mov b, 8
    mov c, 2.5
    mov d, -0.5
    spawn average               | ordered:false delay:0 remap: {first = a, second = b, mean = ab_mean}
    spawn average               | ordered:false delay:0 remap: {first = c, second = d, mean = cd_mean}

; mean = (first + second) / 2
; @0tick
average:
    add mean, first, second
    div mean, 2
```


Solution 2
```nasm two_lights.tasm
_init:
    alias light_a,      c1
    alias light_b,      c2
    alias FAST,         0.25    ; seconds
    alias SLOW,         0.75

    ; parameter of `toggle`
    alias switch,       c9901

    display light_a
    display light_b

_start:
    spawn blink_a
    spawn blink_b

; not 0-tickable: waits
blink_a:
    spawn toggle                | ordered:false delay:0 remap: {switch = light_a}
    waits FAST
    spawn blink_a

; not 0-tickable: waits
blink_b:
    spawn toggle                | ordered:false delay:0 remap: {switch = light_b}
    waits SLOW
    spawn blink_b

; switch = 1 - switch
; @0tick
toggle:
    mul switch, -1
    add switch, 1
```
The two calls use different remaps, so they don't block each other even if they happen on the same tick.


The routines in this chapter live in the same file as the program that uses them. In the next chapter, you will move them into libraries that any program can import.
