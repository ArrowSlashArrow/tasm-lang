---
label: 3. Routines and time
order: 87
---

# Chapter 3: Routines and time
A program usually has more than one routine. In this chapter you will learn how routines start each other, how long instructions take, how to wait, and how to run, pause and stop routines that run side by side.

## 3.1. Ticks
GD runs its game loop 240 times per second. One step of that loop is a **tick**. Everything in TASM is measured in ticks.

When a routine runs, its triggers fire from left to right, one after another. This works because routines are started with GD's **spawn ordered** option, which fires a group's triggers in order of their position:
- In release mode (`-r`), triggers are 1 unit apart and each instruction takes **1 tick**.
- In the default debug mode, triggers are 30 units apart and each instruction takes **30 ticks** (1/8 of a second).

Most instructions take one tick. Some take none (`breakpoint`, `raw`), and the wait instructions take as long as they wait.

## 3.2. Spawning routines
`spawn` starts another routine:

```nasm two_steps.tasm
_init:
    display c1
    display c2

_start:
    mov c1, 1
    spawn second_step
    mov c1, 2

second_step:
    mov c2, 1
```

`spawn` doesn't wait for the routine it starts. `second_step` starts about one tick after the spawn (the spawn trigger's delay), while `_start` carries on with `mov c1, 2`. From then on, the two routines run at the same time. There is no limit to how many routines can run at once.

Routines don't "fall through" into the routine below them. Each routine is its own group, and it only runs when something spawns it. The order of routines in the file doesn't matter.

## 3.3. Loops by spawning
A routine that spawns itself as its last instruction runs again and again. You saw this in [chapter 1](01-getting-started.md). Here are two loops at once, from [`example_programs/two_workers.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/two_workers.tasm):

```nasm two_workers.tasm
; Two routines run at the same time, each counting at its own speed.
; Touch the `stop` block to kill both of them.
_init:
    alias fast_count,   c1
    alias slow_count,   c2
    alias SLOW_DELAY,   6       ; ticks between steps of `slow`

    display fast_count
    display slow_count
    ioblock stop_all, 1, stop   | ordered:false

_start:
    spawn fast
    spawn slow

; not 0-tickable: loops by spawning itself
fast:
    add fast_count, 1
    spawn fast

; not 0-tickable: waits between steps
slow:
    add slow_count, 1
    wait SLOW_DELAY
    spawn slow

stop_all:
    kill fast
    kill slow
```

In release mode, `fast` takes about 2 ticks per step: `add` fires on the first tick, `spawn` on the second, and the next run starts after the spawn's delay of about one tick. `slow` waits 6 slots in between, so a step takes about 8 ticks and it counts roughly four times slower.

`ioblock stop_all, 1, stop` places a second block, labelled **stop**, next to the **start** block. Touching it spawns `stop_all`, which **kills** both loops. [Chapter 8](08-the-level.md) covers IOBlocks in detail. The comments above `fast` and `slow` follow the style guide's rule of marking routines that can't be 0-ticked; [chapter 5](05-fast-code.md) explains what that means.

> [!NOTE]
> A routine that is already running can't be started again: GD ignores the spawn. Spawning a routine from itself works as long as the spawn is the routine's last instruction. Thanks to the spawn delay, the current run has finished by the time the new one starts.

## 3.4. Waiting
| Instruction | Waits |
| ----------- | ----- |
| `nop` | 1 instruction slot |
| `wait n` | `n` instruction slots |
| `waits s` | `s` seconds |

`nop` and `wait` count instruction slots, so they wait 30 times longer in debug mode than in release mode. `waits` converts seconds into slots for the mode you compile in, so it waits the same real time in both. Use `waits` for anything a player should see, and `wait` for short pauses that make one routine wait for another (chapter 9 has examples).

This program blinks: `light` switches between 0 and 1 twice per second.

```nasm blinker.tasm
_init:
    alias light,        c1
    alias BLINK_TIME,   0.5     ; seconds

    display light

; not 0-tickable: waits between blinks
_start:
    ; light = 1 - light, which switches it between 0 and 1
    mul light, -1
    add light, 1
    waits BLINK_TIME
    spawn _start
```

It switches `light` with arithmetic instead of checking its value. That trick is called **branchless** logic, and you will see much more of it in [chapter 5](05-fast-code.md).

> [!IMPORTANT]
> Waits only work in routines that run with spawn ordered. A routine started with `ordered:false` (chapter 5) runs all its instructions in one go and skips its waits.

## 3.5. Pausing, resuming and stopping
These instructions control routines that are already running:

| Instruction | Does |
| ----------- | ---- |
| `pause r` | pauses routine `r` where it is |
| `resume r` | continues routine `r` |
| `kill r` | stops routine `r`. It can't be resumed, but spawning it starts a new run. |

```nasm paused_workers.tasm
; A counter that can be paused, resumed and stopped with three blocks.
_init:
    alias count,        c1
    alias STEP_TIME,    0.25    ; seconds

    display count
    ioblock pause_counting, 1, pause
    ioblock resume_counting, 2, resume
    ioblock stop_counting, 3, stop

_start:
    spawn counting

; not 0-tickable: waits between steps
counting:
    add count, 1
    waits STEP_TIME
    spawn counting

pause_counting:
    pause counting

resume_counting:
    resume counting

stop_counting:
    kill counting
```

A routine can't be paused on the tick it is spawned. Here that is never a problem, because `counting` spends almost all of its time waiting.

[`example_programs/proc_control.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/proc_control.tasm) shows a more unusual use: while a routine is paused, it can be spawned again, so many paused copies of it pile up. A single `resume` releases them all at once.

`toggleoff g` and `toggleon g` switch a group's objects off and on (GD's Toggle trigger). They are useful for objects in the level rather than for routines.

## 3.6. Instructions on the same tick
An instruction with a `~` in front runs on the same tick as the instruction before it:

```nasm
swap:
    ; c1 and c2 swap values in 2 ticks instead of 3
    mov c3, c2
    mov c2, c1
    ~mov c1, c3
```

GD doesn't guarantee the order of triggers that fire on the same tick. `mov c2, c1` and `mov c1, c3` can share a tick because neither reads what the other writes. `mov c3, c2` must come a tick earlier, because `mov c2, c1` overwrites `c2`.

The routine waits as long as the *last* instruction of a `~` group: `wait 3` followed by `~mov c1, 1` waits just 1 tick. Use `~` only where you have checked that the order doesn't matter. [Chapter 5](05-fast-code.md) shows a more reliable way to run many instructions in one tick.

> [!NOTE]
> This feature is intended only for routines that are spawned with spawn-ordered enabled. It is irrelevant for routines that are not spawned ordered.

## 3.7. Exercises
1. Write a traffic light: `light` shows 1 (red) for 3 seconds, 2 (green) for 3 seconds and 3 (yellow) for 1 second, then repeats.
2. Add a **start**-like block labelled `reset` to `paused_workers.tasm` that sets the count back to 0 without stopping it.

Solution 1
Each colour is its own routine, which sets the light, waits, and spawns the next colour:

```nasm traffic_light.tasm
_init:
    alias light,        c1
    alias RED,          1
    alias GREEN,        2
    alias YELLOW,       3
    alias LONG_TIME,    3.0     ; seconds
    alias SHORT_TIME,   1.0

    display light

_start:
    spawn red

; not 0-tickable: waits
red:
    mov light, RED
    waits LONG_TIME
    spawn green

; not 0-tickable: waits
green:
    mov light, GREEN
    waits LONG_TIME
    spawn yellow

; not 0-tickable: waits
yellow:
    mov light, YELLOW
    waits SHORT_TIME
    spawn red
```


Solution 2
Add an IOBlock to `_init` and a routine that resets the counter:

```nasm paused_workers_reset.tasm
_init:
    alias count,        c1
    alias STEP_TIME,    0.25    ; seconds

    display count
    ioblock pause_counting, 1, pause
    ioblock resume_counting, 2, resume
    ioblock stop_counting, 3, stop
    ioblock reset_count, 4, reset

_start:
    spawn counting

; not 0-tickable: waits between steps
counting:
    add count, 1
    waits STEP_TIME
    spawn counting

pause_counting:
    pause counting

resume_counting:
    resume counting

stop_counting:
    kill counting

reset_count:
    mov count, 0
```


All loops so far run forever. In the next chapter, programs start making decisions, so that loops can end.
