---
label: Style guide
icon: paintbrush
order: 80
---

# TASM Style Guide
This guide has two parts:
1. **[Formatting](#part-1-formatting)**: how TASM code should look, so that it is easy to read.
2. **[Writing idiomatic TASM](#part-2-writing-idiomatic-tasm)**: how to write programs that are fast and use few groups. Triggers can be extremely slow and wasteful when they are arranged badly, and very powerful when they are arranged well.

The standard library and the programs in `example_programs/` follow this guide. [`example_programs/collatz.tasm`](../example_programs/collatz.tasm) is a small program that uses most of it.

## Quick checklist
- [ ] Instructions are indented with 4 spaces.
- [ ] Instructions and arguments are lowercase.
- [ ] Flags are aligned in a column after the instruction.
- [ ] Rounding modes are written out in full (`floor`, not `f`).
- [ ] Magic values are given names with `alias`.
- [ ] Instant branches (`ise`, `ife`, ...) are used where timing allows.
- [ ] Branchless logic is used where it saves groups without hurting readability.
- [ ] Routines are 0-ticked (`| ordered:false delay:0`) where possible, and routines that can't be are marked with a comment.
- [ ] Routines receive their inputs through remaps rather than by copying values into temporary items.

## Terms used in this guide
- **Group:** every routine is compiled into one GD group. Some instructions use extra groups (see [Group usage](#1-use-as-few-groups-as-possible)). A level can only use 9,999 groups.
- **Tick:** one step of GD's game loop (1/240th of a second).
- **0-tick:** to run a routine completely within the tick in which it was spawned. See [0-ticking](#2-0-tick-routines-where-possible).
- **Instant branch:** a compare instruction starting with `I` (`ise`, `ifg`, ...). It spawns its target directly, instead of going through an extra spawn trigger.
- **Branchless:** logic that computes a result with arithmetic instead of spawning different routines.
- **Remap:** a `SPAWN` option that swaps one ID for another inside the spawned routine. See [Remaps](#3-pass-arguments-with-remaps).

---

# Part 1: Formatting

## 1. Indent instructions with 4 spaces
```x86asm
routine:
    instruction
    instruction
```

## 2. Write instructions and arguments in lowercase
Instructions, item literals and group literals are case-insensitive, so lowercase is easier to type and read.

```x86asm
    mov c1, c2
    add c3, c4
    spawn g5
```

Built-in names such as `POINTS` and `COLL_P1` must stay uppercase, because the compiler only recognizes them that way.

## 3. Line flags up in a column
Keep flags visibly separate from the instruction. Within a routine, put the `|` of every line in the same column where possible. When an instruction is very long, it's fine to push its flags further to the right.

```x86asm
routine:
    mov c1, c2                  | itemmod:0.5
    fldiv c1, c2, c3            | iter:+ resmode:floor
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
```

## 4. Write rounding modes out in full
For the `*mode` flags (`resmode`, `finmode`, `lmode`, `rmode`), write the full name of the rounding mode. `resmode:floor` is easier to understand than `resmode:f`.

## 5. Name magic values with aliases
A "magic value" is a number or ID whose meaning isn't obvious from the code. Give each one a name in `_init` with `alias`:

```x86asm
_init:
    alias magic_number, 123.45
    alias magic_group,  g1337
    alias magic_item,   t42

_start:
    ise magic_group, magic_item, magic_number
```

## Footnote: Documenting routines
The comment lines directly above a routine (and the comment on its label line) are its documentation, shown when hovering over the routine:

```
; Adds two counters.
; @param c1 first operand
; @param c2 second operand
; @returns c3 the sum
; @0tick
add_two:
    add c3, c1, c2
```
Tags: `@param <name> <description>`, `@returns <name> <description>`, `@0tick` (safe to 0-tick) and `@no0tick <reason>`. A blank `;` line starts a new paragraph.

The example yields the below docs:
### Example routine docs
Adds two counters.
#### Parameters:
* `C1`: first operand
* `C2`: second operand

Returns `C3`: the sum

This routine is safe to 0-tick.

---

# Part 2: Writing idiomatic TASM

## 1. Use as few groups as possible
A level can only have 9,999 groups, and a program shouldn't use all of them: the rest of the level needs some too. Every routine uses one group, and every *non-instant* compare uses one or two extra groups for its intermediate spawn triggers.

### 1.1. Prefer instant branching where you can
Instant branches use no extra groups, and their targets start running sooner:

```x86asm
    ise target, c1, c2      ; instant: 0 extra groups
    se  target, c1, c2      ; regular: 1 extra group
```

The catch is that the target is *not* spawn ordered. Its instructions run in one tick, and waits inside it do nothing. If the target relies on timing (waits, or instructions that must happen on separate ticks), use a regular branch.

### 1.2. Prefer branchless logic
Branches cost groups. Consider a `sign` function, which returns `-1` if `x` is negative, `1` if `x` is positive, and `0` if `x` is 0. A traditional approach would look like this:

```x86asm
; x is in c1, the result goes in c2
sign:
    ife sign_return_zero, sign_check_positive, c1, 0

sign_check_positive:
    ifg sign_return_positive, sign_return_negative, c1, 0

sign_return_positive:
    mov c2, 1

sign_return_negative:
    mov c2, -1

sign_return_zero:
    mov c2, 0
```

This uses **five** groups, one per routine, for a single small function.

The sign function can also be written as `(0 < x) - (x < 0)`: two comparisons and a subtraction. The standard library's `compare` module has branchless comparisons, which return `1` or `0` in `c3` instead of spawning a routine. Using them:

```x86asm
sign:
    mov c2, 0                   ; use the result item to hold 0 for the comparisons
    spawn compare::greater      | ordered:false delay:0     ; c3 = (c1 > c2), i.e. 0 < x
    mov c4, c3                  ; save `0 < x`
    spawn compare::less         | ordered:false delay:0     ; c3 = (c1 < c2), i.e. x < 0
    sub c2, c4, c3              ; c2 = (0 < x) - (x < 0)
```

This uses three groups: `sign`, `compare::greater` and `compare::less`. To get down to a single group, *inline* the comparisons, i.e. copy their instructions into `sign`:

```x86asm
sign:
    mov c2, 0                   ; use the result item to hold 0 for the comparisons
    ; c3 = (c1 > c2): d / (|d| * 2) + 0.5, where d = c1 - c2
    sub c3, c1, c2
    mul c3, c3, 2               | resmode:+ iter:/
    add c3, 0.5
    mov c4, c3                  ; save `0 < x`
    ; c3 = (c1 < c2): d / (|d| * -2) + 0.5
    sub c3, c1, c2
    mul c3, c3, -2              | resmode:+ iter:/
    add c3, 0.5
    sub c2, c4, c3              ; c2 = (0 < x) - (x < 0)
```

Inlining saves the most groups, but the code is much harder to understand and maintain. Only do it when you really need every group.

> [!NOTE]
> `stdlib/compare.tasm` writes these lines as `div c3, c3, 2 | resmode:+ iter:/`. That relies on a compiler bug, where the three-argument form of `div` multiplies (see `unresolved_issues.md`, 4.1). Writing `mul` gives the same result without depending on the bug.

## 2. 0-tick routines where possible
A routine that is **0-ticked** runs completely within the tick in which it was spawned. This is the fastest way to run code: dozens or even hundreds of instructions can run in a single tick.

To 0-tick a routine, spawn it with both `ordered:false` and `delay:0`:
- `ordered:false` turns off spawn ordered, so all of the routine's instructions run at once instead of one tick apart.
- `delay:0` removes the default spawn delay of about one tick.

Instant branches (`ise`, `ife`, ...) also run their target this way.

```x86asm
    spawn modulus               | ordered:false delay:0
```

Whenever a routine *can* be 0-ticked, it should be. If a routine can't be (because it needs specific timing, or calls a routine that can't be 0-ticked), say so in a comment above it:

```x86asm
; computes c3 = c1 mod c2 (from stdlib/math.tasm)
modulus:
    div c3, c1, c2              | resmode:floor
    mul c3, c2
    sub c3, c1, c3

; not 0-tickable: waits between the two additions
increment_with_delay:
    add c1, 1
    waits 1.0                   ; wait 1 second before adding again
    add c1, 1
```

### 2.1. What prevents 0-ticking
A routine won't work correctly when 0-ticked if it:
- **waits.** `nop`, `wait` and `waits` do nothing in a 0-ticked routine, since everything happens in one tick.
- **calls a routine that can't be 0-ticked.**
- **loops or recurses,** i.e. spawns a routine that is already running with the same remaps (see 2.3).

### 2.2. How a 0-ticked call runs
When a routine spawns another routine with `ordered:false delay:0` (or an instant branch), the spawned routine runs **right away**, and the spawner continues once it has finished. The call behaves as if the routine's instructions were pasted in at that point:

```x86asm
runner:
    mov c1, 1
    mov c2, 2
    spawn helper                | ordered:false delay:0
    mov c4, c3

helper:
    mul c3, c1, c2
    div c3, 2
```
behaves like:
```x86asm
runner:
    mov c1, 1
    mov c2, 2
    ; -- start of helper --
    mul c3, c1, c2
    div c3, 2
    ; -- end of helper --
    mov c4, c3
```

This only applies to 0-ticked calls. A normal `spawn` starts the routine about one tick later, while the spawner carries on.

### 2.3. A routine can't 0-tick itself
In a 0-ticked chain, spawn only routines that can be 0-ticked. Also, a routine can only be spawned if it isn't *already running* with the same remaps:

```x86asm
helper:
    mov c1, 1

works:
    spawn helper                | ordered:false delay:0
    spawn helper                | ordered:false delay:0
    spawn helper                | ordered:false delay:0
    ; fine: each call to `helper` finishes before the next one starts,
    ; so there is never more than one copy of it running

doesnt_work:
    spawn helper                | ordered:false delay:0
    spawn doesnt_work           | ordered:false delay:0     ; ignored: `doesnt_work` is already running
```

`doesnt_work` tries to recurse: it spawns itself while it's still running, and GD ignores that spawn.

Adding a remap counts as a different configuration, so it lets the routine be spawned once more, even with a remap that changes nothing in practice (such as `123 = 456`). Spawning it a third time with the same remap is ignored again, since that configuration is already running.

To loop, leave a tick between iterations. Spawn the next iteration with a normal `spawn` (which has a delay of about one tick), so that the current iteration has finished by the time the next one starts. `collatz.tasm` does this in its `next_iteration` routine.

### 2.4. 0-ticking `_start`
The `start` IOBlock spawns `_start` with spawn ordered enabled. To turn that off, compile with `-u` (`--unordered-start`).

## 3. Pass arguments with remaps
To give a routine its inputs, prefer remapping its items over copying values into temporary items:

```x86asm
; uses `modulus` from section 2, which reads c1 and c2 and writes c3
routine:
    ; assume that these aliases are defined in _init
    spawn modulus               | ordered:false delay:0 remap: {c1 = dividend, c2 = divisor, c3 = remainder}
```

Remaps can also save groups, since one routine can be reused for different items instead of being copied.

> [!WARNING]
> A remap swaps **every kind of ID** with that number, not just one kind. `remap: {c1 = c10}` remaps ID 1 to 10 in the spawned routine, which affects counter 1, timer 1 *and* group 1. If the routine (or anything it spawns) uses group 1, that group gets swapped too. Check which groups your routines use (`tasmc --linker-output` lists them), or pass arguments through IDs that no routine uses. `stdlib/vec.tasm`, for example, uses IDs 9991 to 9994.

### 3.1. Remaps carry through nested spawns
Remaps can themselves be remapped:

```x86asm
_start:
    spawn helper1               | remap: {c1 = c2}

helper1:
    spawn helper2               | remap: {c2 = c3}
```

`helper1` sees `c2` wherever it uses `c1`. When it spawns `helper2`, `c2` is remapped again, so `helper2` ends up seeing `c3` wherever it uses `c1`.

## 4. Concurrent instructions
Concurrent instructions (instructions prefixed with a `~`) are unpredictable and generally not recommended for use in programs. They also do not work on routines that are spawned with `ordered:false`.  