---
label: 5. Concurrency and timing
order: 85
---

# Chapter 5: Concurrency and timing
In most languages, running two things at once takes extra work. In TASM, it is the default: every spawn starts a new, independent thread of execution. This chapter shows how to make use of that, and how to control timing precisely.

## 5.1. Running routines side by side
[`two_workers.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/two_workers.tasm) starts two loops that run at the same time:

```x86asm
; Two routines run at the same time, each counting at its own speed.
; Touch the `stop` block to kill both of them.
_init:
    display C1
    display C2
    ioblock stop_all, 2, stop

_start:
    spawn fast
    spawn slow

fast:
    add C1, 1
    spawn fast      ; loop: spawn this routine again

slow:
    add C2, 1
    wait 6          ; sleep for 6 ticks before looping
    spawn slow

stop_all:
    kill fast
    kill slow
```

`_start` spawns both loops and finishes immediately. `fast` and `slow` then run independently, each at its own pace. Neither one knows about the other.

The `ioblock` line in `_init` places a second block in the level, labelled `stop`, which spawns `stop_all` when the player touches it. Chapter 6 covers IOBlocks in detail.

## 5.2. Waiting
Three instructions make a routine wait:
- `NOP` waits for one tick.
- `WAIT n` waits for *n* ticks.
- `WAITS s` waits for *s* seconds.

Waits do not compile to any triggers. They leave an empty gap in the routine, and since GD runs a spawn ordered routine from left to right, the gap becomes a delay.

`WAITS` adjusts itself to the compile mode, so `WAITS 1` waits about one second in both debug and release mode. `NOP` and `WAIT` count steps, so like every other instruction, they are 30 times slower in debug mode.

## 5.3. Pausing, resuming and killing
Three instructions control routines that are already running:
- `PAUSE routine` pauses every running copy of the routine.
- `RESUME routine` resumes every paused copy.
- `KILL routine` stops every running copy for good.

In `two_workers.tasm`, `stop_all` uses `KILL` to stop both loops.

> [!NOTE]
> Killing a loop that respawns itself quickly can be tricky, since the kill may arrive while a new copy is about to start. [`list_search.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/list_search.tasm) kills its loop twice for exactly this reason. If a loop survives a `KILL`, try the same.

Pausing has a useful side effect. [`proc_control.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/proc_control.tasm) uses it to collect many copies of a routine and release them all at once:

```x86asm
_init:
    IOBLOCK continue, 2, cont.
    DISPLAY C1

_start:
    MOV C1, C1      ; dummy op
    NOP             ; wait 2nd tick
    PAUSE _start    ; finally, pause this group
    ADD C1, 1       ; this runs after being unpaused.

continue:
    RESUME _start   ; resume all paused instances
```

Each time you touch `start`, a new copy of `_start` runs and pauses itself. Touch it five times, and five copies are waiting. Touching `cont.` resumes all five at once, and `C1` jumps up by 5.

Note the `MOV` and `NOP` before the `PAUSE`. A routine cannot pause itself immediately after it is spawned, as GD ignores the pause. It needs at least one tick between its first instruction and the `PAUSE`.

## 5.4. Ordered and unordered routines
By default, every routine is spawned with spawn ordered, which runs its instructions from left to right, one step apart. With `ordered:false`, the routine is spawned **without** spawn ordered, and all of its triggers run in the same tick:

```x86asm
_start:
    SPAWN setup | ordered:false

setup:              ; all three instructions run on the same tick
    MOV C1, 1
    MOV C2, 2
    MOV C3, 3
```

This is much faster, but has two consequences:
- Waits inside the routine do nothing.
- GD does not guarantee the order in which triggers run within one tick. Only spawn routines unordered when their instructions do not depend on each other, or when the routine is documented to support it.

The standard library spawns many of its routines unordered for speed. Chapter 7 comes back to this.

## 5.5. Concurrent instructions
You can also run several instructions on the same tick *inside* a routine. Put a `~` in front of an instruction to place it on the same tick as the instruction before it:

```x86asm
sequential:     ; takes 3 ticks
    MOV C1, 1
    MOV C2, 2
    MOV C3, 3

concurrent:     ; takes 1 tick
    MOV C1, 1
    ~MOV C2, 2
    ~MOV C3, 3
```

The same caution applies: instructions on the same tick may run in any order. Here is a swap that goes wrong because of that:

```x86asm
unsafe_swap:
    MOV C3, C2
    ~MOV C2, C1     ; may overwrite C2 before it is copied!
    ~MOV C1, C3     ; may read C3 before it is written!
```

The rule to follow is: **never read and write the same item on the same tick.** A safe swap copies both values out on one tick, and writes both back on the next. It uses one extra temporary item, but still saves a tick compared to three sequential instructions:

```x86asm
swap:
    MOV C3, C1      ; tick 1: only reads C1 and C2...
    ~MOV C4, C2     ; ...and only writes C3 and C4
    MOV C1, C4      ; tick 2: only reads C3 and C4...
    ~MOV C2, C3     ; ...and only writes C1 and C2
```

## 5.6. Timers
Timer items can also be used as actual timers, using GD's Time triggers:
- `TSPAWN timer, start, target, routine` sets the timer to `start`, starts it counting up, and spawns `routine` when it reaches `target` (in seconds).
- `TSTOP timer` pauses the timer, and `TSTART timer` unpauses it.

```x86asm
_start:
    TSPAWN T1, 0, 5, alarm      ; spawn alarm after 5 seconds

alarm:
    MOV C1, 1
```

This is the most accurate way to wait for long periods. Chapter 6 builds a stopwatch with these instructions.

## 5.7. Recursion and the missing call stack
A routine can spawn itself, as the loops above do. Be careful with code that runs *after* a routine has spawned itself, though. GD has no call stack, so there is no way to resume "the copy that called me". Every paused or waiting copy of a routine is the same group, and `RESUME` releases all of them at once.

If you need real recursion (for example, to walk a tree), you will have to build a stack yourself out of items, using the techniques in Chapter 8.

## Summary
- Spawned routines run concurrently and independently.
- `NOP`, `WAIT` and `WAITS` delay a spawn ordered routine.
- `PAUSE`, `RESUME` and `KILL` control all running copies of a routine.
- `ordered:false` and `~` run instructions on the same tick, but the order within a tick is not guaranteed.
- `TSPAWN` uses a timer item to spawn a routine after a period of time.

## Exercises
1. Add a third loop to `two_workers.tasm` that counts down in `C3` every 12 ticks. Make `stop_all` stop it too.
2. Add an IOBlock to `two_workers.tasm` that pauses `fast`, and another that resumes it.
3. Use `TSPAWN` to write a program that adds 1 to `C1` every 2 seconds, forever.
4. Compile `proc_control.tasm` and try it out. What happens if you touch `cont.` before touching `start`?

Next: [Chapter 6: Talking to the level](06-the-level.md).
