---
label: 4. Branching
order: 86
---

# Chapter 4: Branching
Programs need to make decisions: stop a loop when it's done, treat even and odd numbers differently, react to a random roll. In TASM, a decision means *spawning one routine or another*. In this chapter you will learn the compare instructions and use them to write loops that end, the Collatz sequence, a prime checker and a dice roller.

## 4.1. Spawn if
These instructions spawn a routine only if a comparison is true:

| Instruction | Spawns the routine if |
| ----------- | --------------------- |
| `se r, a, b` | `a == b` |
| `sne r, a, b` | `a != b` |
| `sl r, a, b` | `a < b` |
| `sle r, a, b` | `a <= b` |
| `sg r, a, b` | `a > b` |
| `sge r, a, b` | `a >= b` |

`a` is an item; `b` is an item or a number. A countdown that stops at 0:

```nasm countdown.tasm
_init:
    alias remaining,    c1
    alias START,        10
    alias STEP_TIME,    1.0     ; seconds

    display remaining

_start:
    mov remaining, START
    spawn count_down

; not 0-tickable: waits between steps
count_down:
    waits STEP_TIME
    sub remaining, 1
    sg count_down, remaining, 0     ; repeat while remaining > 0
```

The loop condition is the last instruction of the routine, so the routine has finished by the time the next run starts.

## 4.2. Fork: if-else
The `f` versions spawn one routine if the comparison is true and another one if it is false: `fe`, `fne`, `fl`, `fle`, `fg` and `fge`. With a fork, the countdown can do something when it's finished:

```nasm countdown_done.tasm
_init:
    alias remaining,    c1
    alias finished,     c2
    alias START,        10
    alias STEP_TIME,    1.0     ; seconds

    display remaining
    display finished

_start:
    mov remaining, START
    mov finished, 0
    spawn count_down

; not 0-tickable: waits between steps
count_down:
    waits STEP_TIME
    sub remaining, 1
    fg count_down, done, remaining, 0

done:
    mov finished, 1
```

## 4.3. What a compare costs
A compare instruction compiles to an **Item Compare** trigger plus one spawn trigger per target. Those spawn triggers need groups of their own, so each spawn-if uses **1 extra group** and each fork **2 extra groups**, on top of the groups for the routines.

The spawn triggers also keep the target running in spawn ordered, so it can wait. That costs time: a compare takes 1 tick, and its target starts 2 ticks after the compare fires.

Groups are limited (9,999 per level, shared with everything else in the level), so they matter in large programs. [Chapter 5](05-fast-code.md) shows **instant** compares, which use no extra groups.

## 4.4. Comparing modified values
Both sides of a comparison can be changed before they are compared:

| Flag | Does | Default |
| ---- | ---- | ------- |
| `lmod`, `rmod` | number to apply to the left/right side | `1` |
| `lmodop`, `rmodop` | how to apply it: `+`, `-`, `*` or `/` | `*` |
| `lmode`, `rmode` | rounding/sign mode of that side (section 2.5) | none |

Some useful comparisons:

```nasm
modified_compares:
    sg big, c1, 100             | lmode:+                   ; |c1| > 100
    sg half_full, c1, c2        | rmod:2 rmodop:/           ; c1 > c2 / 2
    se even, c1, c1             | lmod:2 lmodop:/ rmod:2 rmodop:/ rmode:floor

big:
    mov c9, 1

half_full:
    mov c9, 2

even:
    mov c9, 3
```

The last line checks whether `c1` is even: `c1 / 2` equals `floor(c1 / 2)` exactly when `c1` is even. This avoids computing a remainder.

> [!WARNING]
> When the right-hand side is a number, GD stores that number *as* the right side's modifier. So `rmod` replaces it: `se r, c1, 10 | rmod:2` compares `c1` with 2. Use `rmod` only when the right side is an item.

## 4.5. Example: Collatz
The [Collatz sequence](https://en.wikipedia.org/wiki/Collatz_conjecture) starts with a number `n` and repeats one rule: if `n` is even, halve it; if it is odd, triple it and add 1. It seems to always reach 1 eventually. This program counts the steps it takes:

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
    ; n is even exactly when n / 2 == floor(n / 2)
    fe even, odd, n, n          | lmod:2 lmodop:/ rmod:2 rmodop:/ rmode:floor

even:
    div n, 2
    spawn count

odd:
    mul n, 3
    add n, 1
    spawn count

count:
    add steps, 1
    sg step, n, 1               ; keep going while n > 1
```

The loop goes `step` → `even` or `odd` → `count` → `step`, and stops when `count` finds `n` is 1. With 27, `n` climbs as high as 9,232 before it comes back down, and `steps` ends at 111.

This version uses 8 groups: 5 for the routines, 2 for the fork in `step` and 1 for the spawn-if in `count`. Plain `spawn`s don't need extra groups. In [chapter 5](05-fast-code.md), the same program gets faster and smaller.

## 4.6. Example: is it prime?
A number is prime if no number other than 1 and itself divides it. This program tries every odd factor up to half of the number. It is based on [`example_programs/is_c1_prime.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/is_c1_prime.tasm).

```nasm is_prime.tasm
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
    fe not_prime, next_factor, n, n | lmod:2 lmodop:/ rmod:2 rmodop:/ rmode:floor

next_factor:
    add factor, 2
    ; no factor up to n / 2 divides n
    fg prime, check_factor, factor, n   | rmod:2 rmodop:/

check_factor:
    ; factor divides n exactly when floor(n / factor) * factor == n
    fldiv multiple, n, factor
    mul multiple, factor
    fe not_prime, next_factor, multiple, n

not_prime:
    mov is_prime, 0

prime:
    mov is_prime, 1
```

The loop alternates between `next_factor` and `check_factor`. The comparison in `next_factor` uses `rmod` on an item, so it compares `factor` with `n / 2`.

## 4.7. Branches that do nothing
A spawn-if only needs one routine. If a fork should do nothing in one of its cases, point that case at a routine that does nothing:

```nasm
at_most_100:
    fg clamp, nothing, c1, 100

clamp:
    mov c1, 100

nothing:
    nop
```

> [!WARNING]
> Give a "do nothing" routine a `nop`. A routine without instructions doesn't get a group, and in TASM v0.3.2 a reference to an empty routine ends up pointing at the routine *below* it.

## 4.8. Randomness
`srand r, chance` spawns `r` with the given chance, in percent. `frand r1, r2, chance` spawns `r1` with that chance and `r2` otherwise:

```nasm coin_flip.tasm
_init:
    alias side,         c1      ; 1 = heads, 2 = tails
    alias HEADS,        1
    alias TAILS,        2

    display side
    ioblock flip_coin, 1, flip

_start:
    spawn flip_coin

flip_coin:
    frand heads, tails, 50

heads:
    mov side, HEADS

tails:
    mov side, TAILS
```

The routine is called `flip_coin` rather than `flip` because an IOBlock label that matches a routine name is read as that routine, which doesn't compile.

For more than two outcomes, chain forks. A die has six sides, so the first roll picks 1 with a chance of 1/6. If it doesn't, the second picks 2 with a chance of 1/5 of the remaining cases, and so on:

```nasm dice.tasm
; Rolls a six-sided die. Touch the `roll` block to roll again.
_init:
    alias die,          c1

    display die
    ioblock roll_die, 1, roll

_start:
    spawn roll_die

roll_die:
    frand one, roll_2_to_6, 16.667

roll_2_to_6:
    frand two, roll_3_to_6, 20

roll_3_to_6:
    frand three, roll_4_to_6, 25

roll_4_to_6:
    frand four, roll_5_to_6, 33.333

roll_5_to_6:
    frand five, six, 50

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

That is 11 routines and 10 extra groups for one die roll. [Chapter 5](05-fast-code.md) shows a cheaper way to make random numbers.

## 4.9. Exercises
1. Store the larger of `a` and `b` in `biggest`.
2. Count how many of the numbers from 1 to 100 are multiples of 7. (Use the remainder pattern from [chapter 2](02-items-and-arithmetic.md#27-example-clock). The answer is 14.)
3. Change `countdown_done.tasm` so that the countdown can be restarted from a block labelled `again`.

Solution 1
```nasm biggest.tasm
_init:
    alias a,            c1
    alias b,            c2
    alias biggest,      c3

    display biggest

_start:
    mov a, 17
    mov b, 42
    fg a_is_bigger, b_is_bigger, a, b

a_is_bigger:
    mov biggest, a

b_is_bigger:
    mov biggest, b
```
[Chapter 5](05-fast-code.md#53-branchless-logic) shows how to do this without branching.


Solution 2
```nasm multiples_of_7.tasm
_init:
    alias n,            c1
    alias hits,         c2
    alias multiple,     c3      ; floor(n / 7) * 7
    alias DIVISOR,      7
    alias LIMIT,        100

    display n
    display hits

_start:
    mov n, 0
    mov hits, 0
    spawn next_number

next_number:
    add n, 1
    mov multiple, n
    fldiv multiple, DIVISOR
    mul multiple, DIVISOR
    se hit, multiple, n
    sl next_number, n, LIMIT

hit:
    add hits, 1
```
`hit` runs alongside `next_number`. They don't write the same items, so it doesn't matter which one runs first.


Solution 3
```nasm countdown_again.tasm
_init:
    alias remaining,    c1
    alias finished,     c2
    alias START,        10
    alias STEP_TIME,    1.0     ; seconds

    display remaining
    display finished
    ioblock restart, 1, again

_start:
    mov remaining, START
    mov finished, 0
    spawn count_down

; not 0-tickable: waits between steps
count_down:
    waits STEP_TIME
    sub remaining, 1
    fg count_down, done, remaining, 0

done:
    mov finished, 1

restart:
    kill count_down
    spawn _start
```
`restart` stops a countdown that may still be running before it starts a new one. Otherwise, touching `again` mid-countdown could leave two copies of the loop counting down the same item.


All the programs in this chapter work, but they are slow: every branch costs a couple of ticks and one or two groups. The next chapter is about making programs fast.
