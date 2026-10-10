---
label: 6. Talking to the level
order: 84
---

# Chapter 6: Talking to the level
A program is not much use if nobody can see what it does or interact with it. This chapter covers the instructions that connect your program to the level and the player: displays, IOBlocks, timers, collisions, events, and persistent items.

## 6.1. Showing values with DISPLAY
You have been using `DISPLAY` since Chapter 1. It places a counter label in the level that shows the value of an item:

```x86asm
_init:
    DISPLAY C1
    DISPLAY T1
    DISPLAY POINTS
```

Labels are placed to the left of the level start, stacked upwards in the order that you declare them. `DISPLAY` may only be used in `_init`.

A good habit while developing is to `DISPLAY` every item that your program uses. It is the easiest way to see what is going on.

## 6.2. Input with IOBlocks
GD has no keyboard input for triggers, so TASM uses **IOBlocks** instead: blocks along the bottom of the level, each with a label and a touch-activated Spawn trigger. When the player walks into one, it spawns a routine.

```x86asm
_init:
    IOBLOCK reset, 2, reset counters
```

The three arguments are:
1. The routine to spawn.
2. The block's position. Position 0 is the automatic `start` block, so start from 1 or 2.
3. A label. It may contain spaces, but not commas, `;` or `|`.

`IOBLOCK` may only be used in `_init`.

> [!NOTE]
> If a label looks like another kind of value, such as `g12` or `5`, put a `\` in front of it: `IOBLOCK go, 3, \5`. Avoid labels that are the same as a routine name.

### A worked example: a stopwatch
[`stopwatch.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/stopwatch.tasm) combines IOBlocks with the timer instructions from Chapter 5:

```x86asm
; A stopwatch built on a timer item, controlled with IOBlocks.
_init:
    alias seconds, T1
    alias minutes, C1

    display minutes
    display seconds

    ioblock pause_watch, 2, pause
    ioblock resume_watch, 3, resume
    ioblock reset_watch, 4, reset

_start:
    ; start T1 at 0 and spawn `next_minute` when it reaches 60
    tspawn seconds, 0, 60, next_minute

next_minute:
    add minutes, 1
    tspawn seconds, 0, 60, next_minute

pause_watch:
    tstop seconds

resume_watch:
    tstart seconds

reset_watch:
    mov minutes, 0
    tspawn seconds, 0, 60, next_minute
```

`T1` counts up in real time. Each time it reaches 60 seconds, `next_minute` adds a minute and restarts the timer at 0. The three IOBlocks pause, resume and reset the stopwatch.

`TSPAWN` has a few flags of its own. For example, `tmod` changes the timer's speed, so `tspawn seconds, 0, 60, next_minute | tmod:2` makes the stopwatch run twice as fast. See [TSPAWN](../index.md#tspawn) in the reference for all of them.

## 6.3. Collisions
Collision instructions react to the player or to collision blocks touching each other. Collision blocks are written as `b` followed by the block ID (`b1`, `b2`, ...), and the players are written as `COLL_P1`, `COLL_P2` or `COLL_P_ANY` (either player).

- `COLL routine, hitbox1, hitbox2` keeps watching, and spawns the routine whenever the two hitboxes collide.
- `INSTCOLL yes, no, hitbox1, hitbox2` checks once, right now, and spawns `yes` or `no`.

[`coin_counter.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/coin_counter.tasm) counts how often the player touches a block:

```x86asm
; Counts how many times player 1 touches a collision block.
; Before playing, place a collision block with block ID 1 somewhere in the level.
_init:
    alias COIN, b1
    display POINTS

_start:
    coll got_coin, COLL_P1, COIN

got_coin:
    add POINTS, 1
```

After compiling, open the level in the editor and place a collision block with block ID 1 somewhere the player can reach. Playtest, touch `start` to begin watching for collisions, then walk into the block.

Only some pairs of hitboxes are allowed, and they must be in this order:

| Hitbox 1 | Hitbox 2 |
| -------- | -------- |
| Collision block | Collision block |
| `COLL_P1`, `COLL_P2` or `COLL_P_ANY` | Collision block |
| `COLL_P1` | `COLL_P2` |

Any other combination is a compile error (E0048).

## 6.4. Events
`EVENT` spawns a routine whenever a gameplay event happens, such as the player jumping or landing. It takes a routine and an event ID, and may only be used in `_init`:

```x86asm
_init:
    EVENT on_event, 1   ; replace 1 with the event ID you want
```

GD has many events, each with its own ID. See [EVENT](../index.md#event) in the reference for the list of IDs, and for the `extra1` and `extra2` flags that some events use.

## 6.5. Persistent items
Normally, every item resets to 0 when the player dies and starts a new attempt. `PERS` makes an item **persistent**, so that it keeps its value across attempts:

```x86asm
_init:
    PERS C1         ; C1 survives deaths
    DISPLAY C1
```

`UNPERS` undoes this for one item, `UNPERSALL` undoes it for every item, and `RPERSALL` resets all persistent items to 0. These instructions can be used in any routine, not just `_init`.

## 6.6. Toggling groups
`TOGGLEON` and `TOGGLEOFF` turn a group on or off. Since collision blocks and other objects can also be in groups, this lets your program enable and disable parts of the level:

```x86asm
close_door:
    TOGGLEOFF g500  ; g500 is a group ID used by objects in the level
```

`g500` is a **group literal**: a fixed group ID, as opposed to a routine name, whose group the compiler chooses. Use group literals for objects that you placed in the level yourself. If your program uses group literals, make sure that they do not clash with the groups that the compiler assigns to routines. The `--group-offset` compiler option moves the routine groups out of the way: with `--group-offset 1000`, routines start at group 1001.

## 6.7. Placing your own objects
For triggers that TASM does not have an instruction for, there are two escape hatches. Both take a GD object string, which you can copy from the editor with the BetterEdit mod.
- `RAW` places an object anywhere in the level, unrelated to the routine.
- `RAWTRG` places a trigger *inside* the routine, in the current instruction's position, as if it were an instruction.

The standard library's `debug` module uses `RAWTRG` to flash the background with a Pulse trigger:

```x86asm
__ok:   ; pulse green
    rawtrg 1,1006,2,105,3,255,36,1,51,1000,47,0.2,210,1,7,0,8,255,9,0
```

> [!WARNING]
> `;` starts a comment, so an object string can only contain one object. Leave out the trailing `;` that copied object strings end with.

## Summary
- `DISPLAY` shows an item's value in the level.
- `IOBLOCK` lets the player spawn routines by touching blocks.
- `COLL` and `INSTCOLL` react to collisions between players and collision blocks.
- `EVENT` reacts to gameplay events.
- `PERS` keeps items across attempts.
- `TOGGLEON`/`TOGGLEOFF` control groups in the level, and `RAW`/`RAWTRG` place any object.

## Exercises
1. Add a `lap` IOBlock to `stopwatch.tasm` that copies the current time into `T2`. Display `T2`.
2. Make `coin_counter.tasm` keep its score across deaths.
3. Extend `coin_counter.tasm` with a second collision block, `b2`, that subtracts a point instead.
4. Write a program that uses `INSTCOLL` to set `C1` to 1 if player 1 is touching `b1` when the `start` block is touched, and to 0 otherwise.

Next: [Chapter 7: Modules and the standard library](07-modules.md).
