---
label: 8. Talking to the level
order: 82
---

# Chapter 8: Talking to the level
So far, programs have only talked to themselves. In this chapter you will connect them to the level: show values, add buttons, measure time, react to the player touching things and pressing keys, and keep values between attempts.

## 8.1. Displays
`display item` places a counter display that shows an item's value. Displays are placed to the left of the start, stacked upwards in the order they are declared. `display` is only allowed in `_init`.

Any item can be displayed, including timers and `POINTS`, `ATTEMPTS` and `MAINTIME`:

```nasm displays.tasm
_init:
    display ATTEMPTS            ; bottom
    display MAINTIME
    display POINTS              ; top

_start:
    add POINTS, 10
```

## 8.2. IOBlocks
An **IOBlock** is a block with a label and a touch trigger that spawns a routine. The **start** block is one, and you can add your own:

```text
ioblock routine, position, label
```

- `position` is a whole number. The blocks are placed in a row at the bottom of the level, and `position` picks the slot. Slot 0 is the **start** block, so use 1 and up.
- `label` is the text on the block. It can't contain commas, `;` or `|`. If it looks like another value (for example `g12` or `5`), put a `\` in front of it.
- A label can't be the name of a routine or an alias; it would be read as that routine or alias.
- The touch trigger uses spawn ordered. Add `| ordered:false` to run the routine in one go.

IOBlocks are only allowed in `_init`. They are meant for testing, and as simple buttons for players.

## 8.3. Timers
A timer item can count time on its own. `tspawn` starts a timer and spawns a routine when it reaches a target value:

```text
tspawn timer, start_value, target_value, routine
```

| Instruction / flag | Does |
| ------------------ | ---- |
| `tstop timer` | pauses the timer |
| `tstart timer` | continues a paused timer |
| `tmod:x` | the timer runs `x` times as fast (negative values count down) |
| `tstop:true` | the timer stops when it reaches the target. This option is needed for the timer to actually spawn the target group. |
| `tpaused:true` | the timer starts paused |
| `nover:true` | doesn't restart a timer that is already running |

This stopwatch is [`example_programs/stopwatch.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/stopwatch.tasm). `seconds` counts from 0 to 60, and each time it gets there, `next_minute` adds a minute and starts it again:

```nasm stopwatch.tasm
; A stopwatch built on a timer item, controlled with IOBlocks.
_init:
    alias seconds,      t1
    alias minutes,      c1
    alias MINUTE,       60

    display minutes
    display seconds

    ioblock pause_watch, 1, pause
    ioblock resume_watch, 2, resume
    ioblock reset_watch, 3, reset

_start:
    tspawn seconds, 0, MINUTE, next_minute

next_minute:
    add minutes, 1
    tspawn seconds, 0, MINUTE, next_minute

pause_watch:
    tstop seconds

resume_watch:
    tstart seconds

reset_watch:
    mov minutes, 0
    tspawn seconds, 0, MINUTE, next_minute
```

A timer can also limit how long something may take. This program counts how many times the player presses jump in 10 seconds, using `MAINTIME` as the clock (section 8.5 explains `event`):

```nasm jump_test.tasm
; Counts jumps during the first 10 seconds of the level.
_init:
    alias jumps,        c1
    alias JUMP_PRESSED, 69      ; event ID
    alias TIME_LIMIT,   10      ; seconds

    display jumps
    display MAINTIME
    event count_jump, JUMP_PRESSED

_start:
    mov jumps, 0

; spawned by the event trigger
count_jump:
    isl add_jump, MAINTIME, TIME_LIMIT

add_jump:
    add jumps, 1
```

## 8.4. Collisions
TASM can react to hitboxes touching. A hitbox is either a **collision block** with an ID, written `b1` to `b9999`, or a player: `COLL_P1`, `COLL_P2` or `COLL_P_ANY` (either player). Place collision blocks in the editor and give them the IDs your program uses.

Only these pairs are allowed, in this order (anything else is error E0048):

| First | Second |
| ----- | ------ |
| collision block | collision block |
| `COLL_P1`, `COLL_P2` or `COLL_P_ANY` | collision block |
| `COLL_P1` | `COLL_P2` |

`coll routine, a, b` places a **Collision** trigger: from the moment it runs, it spawns the routine whenever the two hitboxes start touching. This is [`example_programs/coin_counter.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/coin_counter.tasm):

```nasm coin_counter.tasm
; Counts how many times player 1 touches a collision block.
; Before playing, place a collision block with block ID 1 somewhere in the level.
_init:
    alias COIN,         b1

    display POINTS

_start:
    coll got_coin, COLL_P1, COIN

got_coin:
    add POINTS, 1
```

`instcoll yes, no, a, b` checks *once*, at the moment it runs: it spawns `yes` if the hitboxes touch and `no` if they don't. Both routines are required. This program checks whether the player is standing in a zone when they jump:

```nasm zone_check.tasm
; Counts jumps made inside and outside a zone.
; Before playing, place a collision block with block ID 2 to mark the zone.
_init:
    alias inside_count,     c1
    alias outside_count,    c2
    alias ZONE,             b2
    alias JUMP_PRESSED,     69

    display inside_count
    display outside_count
    event check_zone, JUMP_PRESSED

_start:
    mov inside_count, 0
    mov outside_count, 0

; spawned by the event trigger
check_zone:
    instcoll inside, outside, COLL_P1, ZONE

inside:
    add inside_count, 1

outside:
    add outside_count, 1
```

## 8.5. Input and other events
`event routine, id` places an **Event** trigger that spawns the routine whenever a game event happens. It is only allowed in `_init`, and it listens from the start of the level, even before the **start** block is touched.

Compiled levels use platformer mode, so the player's inputs are available as events:

| ID | Event |
| -- | ----- |
| 69 | jump pressed |
| 70 | jump released |
| 71 | left pressed |
| 72 | left released |
| 73 | right pressed |
| 74 | right released |

Other IDs cover landings, orbs, pads, portals, checkpoints and more; see the [list of event IDs](https://docs.rs/gdlib/latest/gdlib/cclocallevels/gdobj/structs/enum.Event.html). The `extra1` and `extra2` flags set the trigger's Extra ID options, which some events use.

This program moves a number left and right with the player's keys:

```nasm arrows.tasm
; Left and right move `position`; jump puts it back to 0.
_init:
    alias position,         c1
    alias JUMP_PRESSED,     69
    alias LEFT_PRESSED,     71
    alias RIGHT_PRESSED,    73

    display position
    event move_left, LEFT_PRESSED
    event move_right, RIGHT_PRESSED
    event recenter, JUMP_PRESSED

_start:
    mov position, 0

; spawned by the event triggers
move_left:
    sub position, 1

move_right:
    add position, 1

recenter:
    mov position, 0
```

## 8.6. Keeping values between attempts
When the player restarts the level, items go back to 0. `pers item` makes an item **persistent**: it keeps its value between attempts. `unpers item` undoes that, `rpersall` resets all persistent items to 0, and `unpersall` makes them all normal again.

This program counts jumps per attempt, and keeps the best count with a branchless maximum from [chapter 5](05-fast-code.md#53-branchless-logic):

```nasm jump_record.tasm
; Counts jumps in this attempt, and remembers the most jumps in any attempt.
_init:
    alias jumps,            c1
    alias best,             c2
    alias difference,       c3
    alias JUMP_PRESSED,     69

    pers best
    display jumps
    display best
    event count_jump, JUMP_PRESSED

_start:
    mov jumps, 0

; best = max(best, jumps) = (best + jumps + |jumps - best|) / 2
; @0tick
count_jump:
    add jumps, 1
    sub difference, jumps, best | resmode:+
    add best, jumps
    add best, difference
    div best, 2
```

## 8.7. Level objects
Routines aren't the only things with groups. In the editor, you can give your own objects a group and control it from TASM:
- `toggleoff g` hides a group's objects and disables its triggers; `toggleon g` brings them back.
- `spawn g`, `pause g`, `resume g` and `kill g` work on any group with triggers.

```nasm door.tasm
; Opens and closes a door made of blocks.
; Before playing, give the door's blocks group 500 in the editor.
_init:
    alias DOOR,         g500

    ioblock open_door, 1, open
    ioblock close_door, 2, close

_start:
    toggleon DOOR

open_door:
    toggleoff DOOR

close_door:
    toggleon DOOR
```

Routines use groups from 1 upwards, so give your own objects high group IDs, or compile with `--group-offset` to make the routines start higher.

For triggers TASM has no instruction for, `raw` inserts an object string copied from the editor (for example with BetterEdit's copy), and `rawtrg` inserts one as an instruction of the routine. [`stdlib/debug.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/stdlib/debug.tasm) uses `rawtrg` to place the Pulse triggers that flash the background:

```nasm
; pulse the background green
pulse_green:
    rawtrg 1,1006,2,105,3,255,36,1,51,1000,47,0.2,210,1,7,0,8,255,9,0
```

Object strings are hard to read, so comment them well. Since `;` starts a comment, each `raw` or `rawtrg` can only hold a single object.

## 8.8. Exercises
1. Add a **lap** block to the stopwatch that copies the current seconds into a second timer, `lap_time`.
2. Change `coin_counter.tasm` so that coins only count during the first 30 seconds.
3. Extend `arrows.tasm` so that `position` stays between -5 and 5. (Hint: [chapter 5, exercise 2](05-fast-code.md#59-exercises).)

Solution 1
```nasm stopwatch_lap.tasm
_init:
    alias seconds,      t1
    alias lap_time,     t2
    alias minutes,      c1
    alias MINUTE,       60

    display minutes
    display seconds
    display lap_time

    ioblock pause_watch, 1, pause
    ioblock resume_watch, 2, resume
    ioblock reset_watch, 3, reset
    ioblock save_lap, 4, lap

_start:
    tspawn seconds, 0, MINUTE, next_minute

next_minute:
    add minutes, 1
    tspawn seconds, 0, MINUTE, next_minute

pause_watch:
    tstop seconds

resume_watch:
    tstart seconds

reset_watch:
    mov minutes, 0
    tspawn seconds, 0, MINUTE, next_minute

save_lap:
    mov lap_time, seconds
```

Solution 2
```nasm timed_coins.tasm
; Counts coins touched during the first 30 seconds.
; Before playing, place a collision block with block ID 1 somewhere in the level.
_init:
    alias COIN,         b1
    alias TIME_LIMIT,   30      ; seconds

    display POINTS
    display MAINTIME

_start:
    coll got_coin, COLL_P1, COIN

got_coin:
    isl add_point, MAINTIME, TIME_LIMIT

add_point:
    add POINTS, 1
```

Solution 3
```nasm arrows_clamped.tasm
; Left and right move `position` between -5 and 5; jump puts it back to 0.
_init:
    alias position,         c1
    alias difference,       c2
    alias limit,            c3
    alias JUMP_PRESSED,     69
    alias LEFT_PRESSED,     71
    alias RIGHT_PRESSED,    73
    alias LOW,              -5
    alias HIGH,             5

    display position
    event move_left, LEFT_PRESSED
    event move_right, RIGHT_PRESSED
    event recenter, JUMP_PRESSED

_start:
    mov position, 0

move_left:
    sub position, 1
    spawn clamp                 | ordered:false delay:0

move_right:
    add position, 1
    spawn clamp                 | ordered:false delay:0

recenter:
    mov position, 0

; position = min(max(position, LOW), HIGH)
; @0tick
clamp:
    mov limit, LOW
    sub difference, position, limit    | resmode:+
    add position, limit
    add position, difference
    div position, 2
    mov limit, HIGH
    sub difference, position, limit    | resmode:+
    add position, limit
    sub position, difference
    div position, 2
```

In the next chapter, you will read and write items whose ID is only known while the program runs.
