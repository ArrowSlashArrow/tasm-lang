---
label: 5. Fast code
order: 85
---

# Chapter 5: Fast code
The programs in the previous chapters spend most of their time waiting: one tick per instruction, a tick of delay per spawn, two ticks per branch. In this chapter you will learn the three techniques from [part 2 of the style guide](../style-guide.md#part-2-writing-idiomatic-tasm) that remove that waiting and save groups:
1. **0-ticking**: running a routine completely within one tick.
2. **Instant branches**: compares that run their target right away and use no extra groups.
3. **Branchless logic**: computing a result with arithmetic instead of choosing between routines.

## 5.1. 0-ticking a routine
Two flags of `spawn` control how the spawned routine runs:
- `ordered:false` turns off spawn ordered. All the routine's instructions run in one go, in the same tick.
- `delay:0` removes the spawn's delay of about one tick.

With both, the routine is **0-ticked**: it runs completely within the tick in which it was spawned. Even better, it runs *right away*: the spawning routine continues only once the spawned one has finished. A 0-ticked spawn behaves as if the routine's instructions were pasted in at that point:

```nasm call_order.tasm
_init:
    display c1
    display c2
    display c4

_start:
    mov c1, 3
    mov c2, 4
    spawn hypotenuse_squared    | ordered:false delay:0
    mov c4, c3                  ; c3 is already 25 here

; c3 = c1² + c2²
; @0tick
hypotenuse_squared:
    mul c3, c1, c1
    mul c3, c2, c2              | iter:+
```

The `@0tick` comment is a convention from the standard library. It tells readers that the routine is safe to 0-tick.

Dozens or even hundreds of instructions can run in one tick this way, which is why it is the most important technique in this chapter. The style guide asks you to 0-tick every routine that can be.

## 5.2. Instant branches
Every compare from [chapter 4](04-branching.md) has an **instant** version with an `i` in front: `ise`, `isne`, `isl`, `isle`, `isg`, `isge`, `ife`, `ifne`, `ifl`, `ifle`, `ifg`, `ifge`, `isrand` and `ifrand`. They take the same arguments and the same comparison flags.

An instant branch spawns its target directly, without the intermediate spawn trigger. So:
- it uses **no extra groups**.
- the target runs right away, like a 0-ticked spawn.
- the target is *not* spawn ordered, so its waits do nothing. Use the normal version when the target needs to wait.

```nasm instant_branch.tasm
_init:
    alias value,        c1
    alias magnitude,    c2      ; 1 if |value| >= 100, 0 if not

    display value
    display magnitude

_start:
    mov value, -250
    mov magnitude, 0
    isge large, value, 100      | lmode:+
    add value, 1                ; runs after `large` has finished

; @0tick
large:
    mov magnitude, 1
```

## 5.3. Branchless logic
A branch needs at least one routine for each outcome. Often, the same result can be computed with arithmetic. That keeps everything in one routine, uses no extra groups, and is easy to 0-tick. These are the most useful building blocks.

**Absolute value and switching between 0 and 1**, which you have already seen:

```nasm
building_blocks:
    mov c1, c1                  | resmode:+     ; c1 = |c1|
    mul c2, -1                  ; c2 = 1 - c2: switches c2 between 0 and 1
    add c2, 1
```

**Larger and smaller of two numbers.** `max(a, b) = (a + b + |a - b|) / 2` and `min(a, b) = (a + b - |a - b|) / 2`:

```nasm min_max.tasm
_init:
    alias a,            c1
    alias b,            c2
    alias larger,       c3
    alias smaller,      c4
    alias difference,   c5      ; |a - b|

    display larger
    display smaller

_start:
    mov a, 17
    mov b, 42
    spawn min_max               | ordered:false delay:0

; larger = max(a, b), smaller = min(a, b)
; @0tick
min_max:
    sub difference, a, b        | resmode:+
    add larger, a, b
    add larger, difference
    div larger, 2
    add smaller, a, b
    sub smaller, difference
    div smaller, 2
```

**Sign.** GD sets the result of a division by zero to 0, so `x / |x|` is -1, 0 or 1:

```nasm sign.tasm
_init:
    alias x,            c1
    alias abs_x,        c2
    alias sign,         c3      ; -1, 0 or 1

    display x
    display sign

_start:
    mov x, -7
    spawn get_sign              | ordered:false delay:0

; sign = -1 if x < 0, 0 if x == 0, 1 if x > 0
; @0tick
get_sign:
    mov abs_x, x                | resmode:+
    mov sign, x
    div sign, abs_x
```

Compare this with the style guide's first `sign` routine, which uses five routines and five groups for the same result.

**Is it a multiple?** For `0 <= r < d`, `ceil(r / d)` is 0 if `r` is 0 and 1 otherwise. With `r = n mod d`, that tells you whether `d` divides `n`:

```nasm
; flag = 1 if divisor divides n, otherwise 0. Assumes n >= 0 and divisor > 0.
; @0tick
is_multiple:
    fldiv c3, c1, c2            ; c3 = floor(n / divisor)
    mul c3, c2
    sub c3, c1, c3              ; c3 = n mod divisor
    div c3, c2                  | finmode:ceil  ; 0 if n mod divisor == 0, otherwise 1
    mul c3, -1                  ; 1 - c3
    add c3, 1
```

Results like these, which are always 0 or 1, are useful because you can multiply by them: `total + n * flag` adds `n` only when the flag is 1.

Branchless code can be much harder to read than a branch. Use it where it saves groups or time, and explain the trick in a comment.

## 5.4. What can't be 0-ticked
A routine can't be 0-ticked if it:
- **waits.** All its instructions run in the same tick, so `nop`, `wait` and `waits` do nothing.
- **spawns a routine that can't be 0-ticked.**
- **loops or recurses.** A routine can't be spawned while it is already running (with the same remaps, see [chapter 6](06-functions-and-remaps.md)). A 0-ticked routine that spawns itself is still running, so GD ignores the spawn.

To loop quickly, run each iteration in one tick and leave a tick between iterations. A normal `spawn` or compare with `ordered:false` does exactly that: it keeps the delay of about one tick, but runs the whole routine at once.

This program computes 10! = 1 × 2 × ... × 10. It is a corrected version of [`example_programs/loop.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/loop.tasm):

```nasm factorial.tasm
_init:
    alias product,      c1
    alias i,            c2
    alias N,            10

    display product
    display i

_start:
    mov product, 1
    mov i, 1
    spawn multiply_step         | ordered:false

; not 0-tickable: loops by spawning itself.
; each step runs within a single tick.
multiply_step:
    mul product, i
    add i, 1
    sle multiply_step, i, N     | ordered:false
```

The result is 3,628,800 after 10 steps of about two ticks each.

When the number of iterations is known in advance, you can also **unroll** the loop: spawn a 0-tickable step routine once per iteration. Each 0-ticked spawn finishes before the next one starts, so the routine is never spawned while it is running, and the whole computation takes a single tick:

```nasm factorial_unrolled.tasm
_init:
    alias product,      c1
    alias i,            c2

    display product

_start:
    mov product, 1
    mov i, 1
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0
    spawn multiply_step         | ordered:false delay:0

; @0tick
multiply_step:
    mul product, i
    add i, 1
```

[`bigint.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/bigint.tasm) unrolls a 40-step loop this way. Its division runs in a single tick.

> [!NOTE]
> `_start` is spawned by the **start** block with spawn ordered. To run `_start` itself in one go, compile with `-u` (`--unordered-start`).

## 5.5. Example: Collatz, fast
Here is the Collatz program from [chapter 4](04-branching.md#45-example-collatz), rewritten with instant branches. It is [`example_programs/collatz.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/collatz.tasm), the example the style guide recommends:

```nasm collatz.tasm
; Counts how many steps the Collatz sequence takes to reach 1.
; Rule: if n is even, n = n / 2. If n is odd, n = 3n + 1.
_init:
    alias n,            c1
    alias steps,        c2
    alias START_VALUE,  27      ; 27 takes 111 steps

    display n
    display steps

_start:
    mov n, START_VALUE
    mov steps, 0
    spawn step

step:
    ; n is even exactly when n / 2 == floor(n / 2).
    ; both sides are divided by 2, and the right side is also floored.
    ife even, odd, n, n         | lmod:2 lmodop:/ rmod:2 rmodop:/ rmode:floor

even:
    div n, 2
    spawn count

odd:
    mul n, 3
    add n, 1
    spawn count

count:
    add steps, 1
    isg step, n, 1              ; keep going while n > 1
```

What changed:
- `fe` became `ife` and `sg` became `isg`. The program now uses 5 groups instead of 8.
- `even` and `odd` run right away, as part of `step`.
- `count` uses a normal `spawn`. That leaves the tick between iterations that a loop needs, so `step` is never spawned while it is still running.

## 5.6. Example: Project Euler 1
[Project Euler 1](https://projecteuler.net/problem=1) asks for the sum of all numbers below 1000 that are multiples of 3 or 5. [`example_programs/project_euler_1.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/project_euler_1.tasm) solves it with branches, which run alongside the main loop and need extra waits to avoid race conditions. This version uses the `is_multiple` trick from section 5.3, and runs each number in a single tick:

```nasm euler_1.tasm
; Sums all numbers below 1000 that are multiples of 3 or 5. The answer is 233168.
_init:
    alias n,            c1
    alias total,        c2
    alias divisor,      c3      ; input of `is_multiple`
    alias flag,         c4      ; output of `is_multiple`
    alias by_3,         c5
    alias by_5,         c6
    alias counted,      c7
    alias LIMIT,        999

    display n
    display total

_start:
    mov n, 0
    mov total, 0
    spawn next_number           | ordered:false

; not 0-tickable: loops by spawning itself.
; each number is handled within a single tick.
next_number:
    add n, 1
    mov divisor, 3
    spawn is_multiple           | ordered:false delay:0
    mov by_3, flag
    mov divisor, 5
    spawn is_multiple           | ordered:false delay:0
    mov by_5, flag
    mov divisor, 15
    spawn is_multiple           | ordered:false delay:0
    ; n is counted once if it is a multiple of 3 or 5: by_3 + by_5 - by_15
    add counted, by_3, by_5
    sub counted, flag
    mul counted, n
    add total, counted
    sl next_number, n, LIMIT    | ordered:false

; flag = 1 if divisor divides n, otherwise 0. Assumes n >= 0 and divisor > 0.
; @0tick
is_multiple:
    fldiv flag, n, divisor      ; flag = floor(n / divisor)
    mul flag, divisor
    sub flag, n, flag           ; flag = n mod divisor
    div flag, divisor           | finmode:ceil  ; 0 if n mod divisor == 0, otherwise 1
    mul flag, -1                ; flag = 1 - flag
    add flag, 1
```

It uses 4 groups: 3 routines and 1 for the `sl`. The same routine is spawned three times per number, each time after the previous call has finished.

## 5.7. Example: random numbers
The dice roller in [chapter 4](04-branching.md#48-randomness) needs 11 routines and 10 extra groups. Random numbers get much cheaper when you build them one random bit at a time. This program makes a random number from 0 to 255:

```nasm random_byte.tasm
; Makes a random whole number from 0 to 255. Touch `roll` for a new one.
_init:
    alias value,        c1
    alias weight,       c2      ; value of the current bit: 1, 2, 4, ...

    display value
    ioblock roll_byte, 1, roll

_start:
    spawn roll_byte

; @0tick
roll_byte:
    mov value, 0
    mov weight, 1
    spawn random_bit            | ordered:false delay:0
    spawn random_bit            | ordered:false delay:0
    spawn random_bit            | ordered:false delay:0
    spawn random_bit            | ordered:false delay:0
    spawn random_bit            | ordered:false delay:0
    spawn random_bit            | ordered:false delay:0
    spawn random_bit            | ordered:false delay:0
    spawn random_bit            | ordered:false delay:0

; adds `weight` to `value` with a chance of 50%, then doubles `weight`
; @0tick
random_bit:
    isrand add_weight, 50
    mul weight, 2

; @0tick
add_weight:
    add value, weight
```

Three routines, no extra groups, and every number from 0 to 255 is equally likely. [`example_programs/rng.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/rng.tasm) uses the same idea with timers to make larger numbers.

## 5.8. Counting groups
After compiling, the compiler prints the groups it assigned to routines, e.g. `Using groups 1 - 5`. Extra groups used by compares are not included in that range, so count them yourself: 1 for each spawn-if and `srand`, 2 for each fork and `frand`, and none for instant compares.

`tasmc --linker-output` (`-l`) prints every routine with its group, which helps when a program uses remaps (next chapter).

## 5.9. Exercises
1. Rewrite `dice.tasm` from [chapter 4](04-branching.md#48-randomness) so that it uses no extra groups and a roll takes a single tick.
2. Write a 0-tickable routine that clamps `c1` to the range 0 to 100 without branching. (Hint: `min(max(x, 0), 100)`.)
3. Change `is_prime.tasm` from [chapter 4](04-branching.md#46-example-is-it-prime) so that each factor is checked within a single tick.

Solution 1
Replace every `frand` with `ifrand`:

```nasm dice_instant.tasm
_init:
    alias die,          c1

    display die
    ioblock roll_die, 1, roll

_start:
    spawn roll_die

; @0tick
roll_die:
    ifrand one, roll_2_to_6, 16.667

; @0tick
roll_2_to_6:
    ifrand two, roll_3_to_6, 20

; @0tick
roll_3_to_6:
    ifrand three, roll_4_to_6, 25

; @0tick
roll_4_to_6:
    ifrand four, roll_5_to_6, 33.333

; @0tick
roll_5_to_6:
    ifrand five, six, 50

one:
    mov die, 1

two:
    mov die, 2

three:
    mov die, 3

four:
    mov die, 4

five:
    mov die, 5

six:
    mov die, 6
```


Solution 2
```nasm clamp.tasm
_init:
    alias x,            c1
    alias difference,   c2
    alias limit,        c3      ; holds the bound that x is compared with
    alias LOW,          0
    alias HIGH,         100

    display x

_start:
    mov x, 250
    spawn clamp                 | ordered:false delay:0

; x = min(max(x, LOW), HIGH)
; @0tick
clamp:
    ; x = max(x, LOW) = (x + LOW + |x - LOW|) / 2
    mov limit, LOW
    sub difference, x, limit    | resmode:+
    add x, limit
    add x, difference
    div x, 2
    ; x = min(x, HIGH) = (x + HIGH - |x - HIGH|) / 2
    mov limit, HIGH
    sub difference, x, limit    | resmode:+
    add x, limit
    sub x, difference
    div x, 2
```
`x - LOW` needs an item on both sides, so the bounds are copied into `limit` first.


Solution 3
`check_factor` becomes the target of an instant branch, and the loop is closed by a fork with `ordered:false`, which keeps the tick between iterations:

```nasm is_prime_fast.tasm
; Checks whether `n` is prime. Assumes n >= 3.
_init:
    alias n,            c1
    alias is_prime,     c2      ; 1 if n is prime, 0 if not
    alias factor,       c3
    alias multiple,     c4      ; floor(n / factor) * factor
    alias INPUT,        173

    display n
    display is_prime
    display factor

_start:
    mov n, INPUT
    mov is_prime, 0
    mov factor, 1
    ; even numbers aren't prime
    ife not_prime, next_factor, n, n    | lmod:2 lmodop:/ rmod:2 rmodop:/ rmode:floor

; not 0-tickable: loops through `check_factor`.
; each factor is checked within a single tick.
next_factor:
    add factor, 2
    ; no factor up to n / 2 divides n
    ifg prime, check_factor, factor, n  | rmod:2 rmodop:/

check_factor:
    fldiv multiple, n, factor
    mul multiple, factor
    fe not_prime, next_factor, multiple, n  | ordered:false

not_prime:
    mov is_prime, 0

prime:
    mov is_prime, 1
```


So far, routines have read and written fixed items. In the next chapter, you will turn routines into functions that can work on any items you give them.
