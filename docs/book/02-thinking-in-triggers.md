---
label: 2. Thinking in triggers
order: 88
---

# Chapter 2: Thinking in triggers
TASM looks like assembly, but it does not run on a CPU. It runs on Geometry Dash's trigger system, and that system works differently from the computers that most languages are designed for. This chapter explains the model that every TASM program runs on. It is short, but the rest of the book depends on it.

## 2.1. Routines are groups
Every routine that you write (except `_init`) is compiled into a **group** of triggers. The compiler gives each routine its own group ID, starting from 1:

```x86asm
_start:         ; group 1
    SPAWN worker

worker:         ; group 2
    ADD C1, 1
```

Running a routine means spawning its group, which is exactly what a Spawn trigger does. That is all `SPAWN worker` compiles to: a Spawn trigger that targets group 2.

In the editor, each routine is drawn on its own row, labelled with its group and name, with its triggers laid out from left to right:

```
1: _start   [spawn g2]
2: worker   [item edit C1 += 1]
```

## 2.2. Position is time
GD's Spawn trigger has an option called **spawn ordered**. When it is on, the triggers in the spawned group run one after another according to their x-position, instead of all at once. The compiler turns spawn ordered on by default, and places each instruction one step to the right of the previous one. In release mode, one step is one **tick**, which is 1/240th of a second.

So, in this routine:
```x86asm
setup:
    MOV C1, 10      ; tick 0
    MOV C2, 20      ; tick 1
    ADD C1, C2      ; tick 2
```
the three instructions run on three consecutive ticks, in the order they are written. In debug mode, each step is 30 ticks instead, which is why debug programs run slowly.

> [!NOTE]
> The book measures time in ticks as if you compiled with `--release`. In debug mode, multiply every instruction step by 30.

Most instructions take exactly one tick. Wait instructions such as `WAIT 5` take more, and a few instructions take none. The reference lists the exact time for each instruction.

## 2.3. Spawning does not wait
In most languages, calling a function pauses the caller until the function returns. **Spawning a routine does not pause anything.** The spawner carries on with its next instruction on the next tick, while the spawned routine starts running alongside it:

```x86asm
_start:
    SPAWN worker    ; worker starts running...
    ADD C2, 1       ; ...while _start keeps going

worker:
    ADD C1, 1
```

There is also no call stack, and routines never "return". When a routine reaches its last instruction, it is simply finished. If you need something to happen after a routine is done, that routine has to spawn the next step itself, or the caller has to wait for long enough. You will see both patterns in the next few chapters.

This is the most important difference between TASM and other languages: **every spawn starts a new thread of execution.** Any number of routines can run at the same time, including several copies of the same routine.

## 2.4. Items are the only variables
TASM has no named variables. All data lives in GD's **items**:
- **Counters** (`C1` to `C9999`) hold 32-bit whole numbers.
- **Timers** (`T1` to `T9999`) hold 32-bit decimal numbers.

Items are global. Every routine can read and write every item, so it is up to you to decide which routine uses which item. Chapter 3 shows how **aliases** let you give items readable names.

## 2.5. Everything is limited
GD's IDs are limited to the range 1 to 9,999. This applies to groups, counters, timers and collision blocks. Every routine uses a group, and some instructions use an extra group or two (Chapter 4 explains why), so very large programs can run out of groups. After each compile, the compiler tells you which group IDs it assigned to your routines:

```
Using groups 1 - 5
```

## 2.6. The two special routines
You met these in Chapter 1:
- `_init` contains setup instructions, such as `DISPLAY`. It does not get a group, cannot be spawned, and its triggers run once when the level starts. Several instructions (such as `DISPLAY`, `ALIAS` and `IMPORT`) may *only* be used in `_init`.
- `_start` is the entry point. The compiler places a `start` IOBlock in the level that spawns it.

A routine with no instructions is skipped entirely and does not use a group. This turns out to be useful: an empty routine is a handy target for a branch that should do nothing.

## Summary
- A routine is a group of triggers. Running a routine means spawning its group.
- Instructions run left to right, one step at a time.
- Spawning starts a new, independent thread of execution. Nothing waits and nothing returns.
- Items are the program's only storage, and they are global.
- IDs, including groups, are limited to 1 to 9,999.

## Exercises
1. Compile a program with three routines and look at the rows in the editor. Which group did each routine get?
2. Run `tasmc` with `--linker-output` (or `-l`) on a program. It prints every routine, its group, and its instructions. Compare this output with what you see in the editor.
3. In the example in section 2.3, add a `WAIT 10` instruction between `SPAWN worker` and `ADD C2, 1`. Does this delay `worker` as well? Compile it in debug mode, `DISPLAY` both counters, and watch.

Next: [Chapter 3: Items and arithmetic](03-items-and-arithmetic.md).
