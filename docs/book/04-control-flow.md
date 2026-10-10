---
label: 4. Control flow
order: 86
---

# Chapter 4: Control flow
TASM has no `if` statements or `while` loops. All control flow is done by **spawning routines**, sometimes depending on a condition. This chapter shows how to build branches and loops from that one idea, and how to avoid the race conditions that come with it.

## 4.1. Spawning
You have already seen `SPAWN`: it starts a routine and carries on.

```x86asm
_start:
    SPAWN greet

greet:
    MOV C1, 1
```

`SPAWN` has a few flags. The most common ones are:
- `delay`: wait this many seconds before spawning, e.g. `SPAWN greet | delay:0.5`.
- `ordered`: `true` by default. With `ordered:false`, the routine is spawned without spawn ordered (see [Chapter 5](05-concurrency.md#54-ordered-and-unordered-routines)).
- `remap`: remap IDs in the spawned routine. This is an advanced feature, covered in [Chapter 7](07-modules.md#75-passing-arguments-with-remaps).

## 4.2. Conditional spawns
The **spawn-if** instructions compare two values and spawn a routine only if the comparison is true:

| Instruction | Spawns the routine if |
| ----------- | --------------------- |
| `SE` | a == b |
| `SNE` | a != b |
| `SL` | a < b |
| `SLE` | a <= b |
| `SG` | a > b |
| `SGE` | a >= b |

The first argument is the routine to spawn, followed by the two values. The first value must be an item, and the second may be an item or a number:

```x86asm
check:
    SE  is_zero, C1, 0      ; spawn is_zero if C1 == 0
    SGE big, C1, C2         ; spawn big if C1 >= C2
```

## 4.3. Forks: if/else
The **fork** instructions take two routines. They spawn the first one if the comparison is true, and the second one otherwise. They are named like the spawn-if instructions, but start with `F`: `FE`, `FNE`, `FL`, `FLE`, `FG` and `FGE`.

```x86asm
check:
    FL negative, non_negative, C1, 0    ; if C1 < 0 ... else ...

negative:
    MOV C2, -1

non_negative:
    MOV C2, 1
```

## 4.4. Loops
A loop is a routine that spawns itself while a condition holds. Here is the loop from [`project_euler_6.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/project_euler_6.tasm), which solves [Project Euler problem 6](https://projecteuler.net/problem=6):

```x86asm
_init:
    DISPLAY C1 ; sum of squares
    DISPLAY C2 ; unsquared sum
    DISPLAY C3 ; iterator index (i)
    DISPLAY C4 ; Auxiliary square variable

loop:
    ADD C3, 1 ; increment

    MOV C4, C3
    MUL C4, C4
    ADD C1, C4 ; add squared to c1
    ADD C2, C3 ; add unsquared to c2

    FL loop, finish, C3, 100 ; looooooooooooooop

finish:
    MUL C2, C2 ; square the sum
    SUB C2, C1 ; store result in c2

_start:
    SPAWN loop
```

The `FL` at the end of `loop` works like a `while` loop: while `C3 < 100`, run `loop` again; once it is done, run `finish`. Notice that the fork is the **last** instruction of the routine. That is not a coincidence, as the next section explains.

> [!NOTE]
> A routine may spawn itself, but not as its first instruction. Put at least one other instruction before it.

## 4.5. Race conditions
Remember from Chapter 2 that spawning does not pause the spawner. The same is true for compares: after a compare spawns a routine, the routine that contains the compare **keeps running**. If there are more instructions after the compare, they run at the same time as the routine that was spawned.

This is how a compare runs, tick by tick:
- Tick *n*: the compare is checked.
- Tick *n* + 1: the spawner's next instruction runs.
- Tick *n* + 2: the spawned routine starts.

Look at this code, which tries to add 10 to `C1` only if `C1` is negative, and then double it:

```x86asm
broken:
    SL add_ten, C1, 0
    MUL C1, 2           ; runs before add_ten does!

add_ten:
    ADD C1, 10
```

The `MUL` runs one tick after the compare, but `add_ten` only starts two ticks after the compare. The two routines **race**, and in this case, `MUL` wins every time.

There are two ways to fix this.

**1. Make the branch the last instruction, and continue in a separate routine.** Every path then spawns the rest of the work when it is done. This is the safest pattern, and the one used by most of the examples:

```x86asm
fixed:
    FL add_ten, continue, C1, 0

add_ten:
    ADD C1, 10
    SPAWN continue

continue:
    MUL C1, 2
```

**2. Wait for the branch.** If the branch is short, the spawner can wait until it is done. [`project_euler_2.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/project_euler_2.tasm) uses a `NOP` (a one-tick wait) after a compare for exactly this reason:

```x86asm
    SE add, C5, 0
    NOP ; prevent race conditions
    SL loop, C2, 4000000
```

Waiting is fragile: if the branch gets longer, the wait needs to grow too. Prefer the first pattern when you can.

## 4.6. A worked example: the Collatz sequence
The [Collatz sequence](https://en.wikipedia.org/wiki/Collatz_conjecture) starts from a number *n*. If *n* is even, it is halved; if it is odd, it becomes 3*n* + 1. The sequence always seems to reach 1 eventually. [`collatz.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/collatz.tasm) counts how many steps that takes:

```x86asm
; Counts how many steps the Collatz sequence takes to reach 1.
; Rule: if n is even, n = n / 2. If n is odd, n = 3n + 1.
_init:
    alias n, C1
    alias steps, C2
    alias START_VALUE, 27   ; 27 takes 111 steps

    display n
    display steps

_start:
    mov n, START_VALUE
    mov steps, 0
    spawn step

step:
    ; n is even exactly when n / 2 == floor(n / 2).
    ; both sides are divided by 2, and the right side is also floored.
    fe even, odd, n, n  | lmod:2 lmodop:/ rmod:2 rmodop:/ rmode:floor

even:
    div n, 2
    spawn count

odd:
    mul n, 3
    add n, 1
    spawn count

count:
    add steps, 1
    sg step, n, 1       ; keep going while n > 1
```

The program follows the "branch last, then continue" pattern from section 4.5:
- `step` forks into `even` or `odd`.
- Both branches update `n`, then spawn `count`.
- `count` loops back to `step` while `n > 1`.

The fork in `step` uses **compare flags** to check whether `n` is even. Each side of a compare can be modified before the comparison happens:
- `lmod` and `lmodop` apply an operation to the left side. `lmod:2 lmodop:/` divides it by 2.
- `rmod` and `rmodop` do the same for the right side.
- `lmode` and `rmode` round the side, using the same values as `resmode`.

So `fe even, odd, n, n | ...` compares `n / 2` with `floor(n / 2)`, which are only equal when `n` is even. Without flags, this check would take four instructions (see the modulo example in Chapter 3). The prime checker in [`is_c1_prime.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/is_c1_prime.tasm) uses the same trick.

> [!WARNING]
> When the right side of a compare is a number, `rmod` *replaces* that number: `SE done, C1, 10 | rmod:2` compares `C1` with 2, not with 20. Only use `rmod` when both sides are items.

## 4.7. Instant compares
Each compare instruction has an **instant** version, written with an `I` in front: `ISE`, `IFL`, `IFGE` and so on. Instant compares spawn their target directly, instead of going through an extra Spawn trigger. This makes them faster (the target starts one tick after the compare instead of two) and means they use no extra groups.

The cost is that the target is **not** spawn ordered, so its instructions all run at once and any waits inside it do nothing. Instant compares also ignore the `ordered` flag. Use them for targets that do not depend on timing, such as a routine with a single instruction, or in performance-critical code such as the memory controller in Chapter 8.

## 4.8. Random branches
`SRAND` and `FRAND` spawn a routine based on chance instead of a comparison:

```x86asm
coin_flip:
    FRAND heads, tails, 50      ; 50% heads, otherwise tails
```

[`rng.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/rng.tasm) uses `FRAND` in a loop to build a random number one bit at a time.

## 4.9. How many groups does this use?
Base compares use extra groups for their Spawn triggers, so that the spawned routines can be spawn ordered:

| Instruction | Extra groups |
| ----------- | ------------ |
| Spawn-if (`SE`, ...) and `SRAND` | 1 |
| Fork (`FE`, ...) and `FRAND` | 2 |
| Instant compares | 0 |
| `SPAWN`, arithmetic, waits | 0 |

This rarely matters in small programs, but it adds up in a large program with hundreds of branches.

## Summary
- Spawn-if instructions (`SE`, `SL`, ...) spawn a routine when a condition holds.
- Forks (`FE`, `FL`, ...) choose between two routines.
- Loops are routines that spawn themselves.
- Code after a compare keeps running. Put branches at the end of a routine and continue in a separate routine.
- Compare flags (`lmod`, `rmode`, ...) can transform each side of a comparison.
- Instant compares are faster but not spawn ordered.

## Exercises
1. Change `START_VALUE` in `collatz.tasm` to 97. How many steps does it take?
2. Write a program that counts from 1 to 20 in `C1`, and adds 1 to `C2` each time `C1` is a multiple of 3. (Hint: combine the loop from section 4.4 with a modulo check.)
3. Rewrite the `broken` routine from section 4.5 using the wait pattern. How many ticks of waiting does it need?
4. The prime checker [`is_c1_prime.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/is_c1_prime.tasm) uses several forks. Read through it and work out what each routine does.

Next: [Chapter 5: Concurrency and timing](05-concurrency.md).
