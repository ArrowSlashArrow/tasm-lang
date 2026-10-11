---
label: 9. Memory
order: 81
---

# Chapter 9: Memory
Every item in the programs so far was chosen when the program was written: `c1` is always counter 1. Lists, tables and stacks need more than that. They need to read "the counter whose ID is in `ptr`", where `ptr` changes while the program runs. In this chapter you will use the standard library's **memory controller** to do that.

## 9.1. How the memory controller works
GD has no trigger that reads an item by a computed ID. Remaps can change IDs, though, and [`stdlib/mem_8bit.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/stdlib/mem_8bit.tasm) builds on that:
1. It looks at the address one bit at a time, from 128 down to 1.
2. For each bit that is set, it spawns the next step with a remap that shifts every ID up by that bit's value (for 128: `{1 = 129, 2 = 130, ...}`).
3. At the end, a routine that uses counter 1 or 2 has been shifted onto the counter at the address.

The result: one routine that reads or writes the counter at any address from 1 to 256. A **pointer** item holds the address, and a **register** item holds the value that was read or will be written:

| Alias | Item | Use |
| ----- | ---- | --- |
| `mem_8bit::ptrpos` | `c257` | the address to access (1 to 256) |
| `mem_8bit::cmemreg` | `c258` | the counter register |
| `mem_8bit::tmemreg` | `t258` | the timer register |
| `mem_8bit::temp` | `c259` | used inside the controller |

Each access is a spawn of `mem_8bit::mem`, with `mem_8bit::mem_end` remapped to the operation:

| Operation | Does |
| --------- | ---- |
| `mem_8bit::cread` | `cmemreg` = the counter at `ptrpos` |
| `mem_8bit::cwrite` | the counter at `ptrpos` = `cmemreg` |
| `mem_8bit::tread` | `tmemreg` = the timer at `ptrpos` |
| `mem_8bit::twrite` | the timer at `ptrpos` = `tmemreg` |

## 9.2. Rules for using memory
- **Compile with `--group-offset 128`**. The controller's remaps shift IDs 1 to 256, groups included. With the offset, every routine gets a group from 129 upwards, out of the way of the first, largest shift.
- **Keep your own items above 259**, unless they are meant to be memory. Counters 1 to 256 *are* the memory, and 257 to 259 belong to the controller.
- **Wait after each access.** The call is spawned with the normal spawn delay, so the result is ready about a tick later. The examples wait 2 slots after every access.
- **Spawn `mem` exactly as shown:** `spawn mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}`, with the operation you want.

Because of the waits, a routine that accesses memory can't be 0-ticked.

Address *n* is simply counter *n*: a program can also use `c30` directly, when the address is known in advance. The controller is only needed when it isn't.

## 9.3. First steps
This is [`example_programs/memory_basics.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/memory_basics.tasm). It writes a value to address 10, reads it back, doubles it, and writes the result to address 11:

```nasm memory_basics.tasm
; Reads and writes counters by address, using the 8-bit memory controller.
; Compile with: tasmc memory_basics.tasm --group-offset 128
_init:
    importstd mem_8bit

    alias ptr,          mem_8bit::ptrpos
    alias reg,          mem_8bit::cmemreg
    alias ACCESS_TIME,  2       ; slots to wait after each memory access

    display ptr
    display reg
    display c10
    display c11

; not 0-tickable: waits for the memory controller
_start:
    ; memory[10] = 21
    mov ptr, 10
    mov reg, 21
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cwrite}
    wait ACCESS_TIME

    ; reg = memory[10] * 2
    mov reg, 0
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait ACCESS_TIME
    mul reg, 2

    ; memory[11] = reg
    add ptr, 1
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cwrite}
    wait ACCESS_TIME
```

`c10` ends up as 21 and `c11` as 42.

The aliases `ptr` and `reg` make the program shorter to read. They can be used as arguments, but write the full `mem_8bit::` names inside `remap`.

## 9.4. Example: Fibonacci numbers
The Fibonacci sequence starts with 0 and 1, and each number after that is the sum of the two before it: 0, 1, 1, 2, 3, 5, 8, ... This program fills memory with it, based on [`example_programs/fib_in_memory.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/fib_in_memory.tasm):

```nasm fib_in_memory.tasm
; Fills addresses 1 to 31 with the Fibonacci numbers 0, 1, 1, 2, 3, 5, ...
; Compile with: tasmc fib_in_memory.tasm --group-offset 128
_init:
    importstd mem_8bit

    alias ptr,          mem_8bit::ptrpos
    alias reg,          mem_8bit::cmemreg
    alias older,        c260    ; the first of the two numbers being added
    alias LAST_PAIR,    30      ; the last pair starts at this address
    alias ACCESS_TIME,  2       ; slots to wait after each memory access

    display ptr
    display reg

_start:
    ; memory[1] = 0 already, so only memory[2] needs a value
    mov c2, 1
    mov ptr, 1
    spawn next_number

; memory[ptr + 2] = memory[ptr] + memory[ptr + 1]
; not 0-tickable: waits for the memory controller
next_number:
    ; older = memory[ptr]
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait ACCESS_TIME
    mov older, reg

    ; reg = memory[ptr + 1] + older
    add ptr, 1
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait ACCESS_TIME
    add reg, older

    ; memory[ptr + 2] = reg
    add ptr, 1
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cwrite}
    wait ACCESS_TIME

    ; the next pair starts one address later
    sub ptr, 1
    sle next_number, ptr, LAST_PAIR
```

The original program goes up to address 51. This version stops at address 32, which holds 1,346,269. Going much further would run into the limit from [chapter 2](02-items-and-arithmetic.md): from 24,157,817 (address 38) onwards, the numbers are above 2<sup>24</sup> and may not be exact.

## 9.5. Example: searching a list
A list is a run of consecutive addresses. This program looks for a value in a list and reports the address where it is first found, based on [`example_programs/list_search.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/list_search.tasm):

```nasm list_search.tasm
; Finds the first address in a list that holds `TARGET`.
; Compile with: tasmc list_search.tasm --group-offset 128
_init:
    importstd mem_8bit

    alias ptr,          mem_8bit::ptrpos
    alias reg,          mem_8bit::cmemreg
    alias target,       c260
    alias found_at,     c261    ; address of the first match, or 0 if there is none
    alias LIST_START,   30
    alias LIST_END,     39
    alias TARGET,       9
    alias ACCESS_TIME,  2       ; slots to wait after each memory access

    ; the list: addresses 30 to 39
    mov c30, 1
    mov c31, 2
    mov c32, 3
    mov c33, 5
    mov c34, 9
    mov c35, 2
    mov c36, 5
    mov c37, 1
    mov c38, 56
    mov c39, 2

    display found_at
    display ptr

_start:
    mov target, TARGET
    mov found_at, 0
    mov ptr, LIST_START
    spawn check_item

; not 0-tickable: waits for the memory controller
check_item:
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait ACCESS_TIME
    ife found, next_item, reg, target

; @0tick
found:
    mov found_at, ptr

next_item:
    add ptr, 1
    sle check_item, ptr, LIST_END
```

`found_at` ends up as 34. The loop stops as soon as the value is found, because `found` doesn't spawn `check_item` again. The original program kills the loop from `found` instead, which needs two `kill`s because of timing. Not starting the next iteration is simpler.

The list is written straight into its counters in `_init`, since its addresses are known in advance. `mov` in `_init` runs once when the level starts.

## 9.6. Bigger memory
[`stdlib/mem_14bit.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/stdlib/mem_14bit.tasm) works the same way, with addresses up to 9,999. It needs `--group-offset 8192`, and its routines and registers are documented at the top of the file.

## 9.7. Exercises
1. Add up the list in `list_search.tasm` (the sum is 86).
2. Fill addresses 1 to 20 with the square numbers 1, 4, 9, ..., 400 using `cwrite`.
3. Find the largest number in the list in `list_search.tasm` (it is 56). Use the branchless maximum from [chapter 5](05-fast-code.md#53-branchless-logic).

Solution 1
```nasm list_sum.tasm
; Compile with: tasmc list_sum.tasm --group-offset 128
_init:
    importstd mem_8bit

    alias ptr,          mem_8bit::ptrpos
    alias reg,          mem_8bit::cmemreg
    alias total,        c260
    alias LIST_START,   30
    alias LIST_END,     39
    alias ACCESS_TIME,  2

    mov c30, 1
    mov c31, 2
    mov c32, 3
    mov c33, 5
    mov c34, 9
    mov c35, 2
    mov c36, 5
    mov c37, 1
    mov c38, 56
    mov c39, 2

    display total

_start:
    mov total, 0
    mov ptr, LIST_START
    spawn add_item

; not 0-tickable: waits for the memory controller
add_item:
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait ACCESS_TIME
    add total, reg
    add ptr, 1
    sle add_item, ptr, LIST_END
```

Solution 2
```nasm squares.tasm
; Compile with: tasmc squares.tasm --group-offset 128
_init:
    import stdlib/mem_8bit

    alias ptr,          mem_8bit::ptrpos
    alias reg,          mem_8bit::cmemreg
    alias LAST_ADDRESS, 160
    alias ACCESS_TIME,  2

    display ptr
    display reg

_start:
    mov ptr, 1
    spawn write_square                  | ordered:false

; not 0-tickable: waits for the memory controller
write_square:
    mul reg, ptr, ptr
    spawn mem_8bit::mem                 | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cwrite}
    add ptr, 1
    sle write_square, ptr, LAST_ADDRESS | ordered:false
```

Solution 3
```nasm list_max.tasm
; Compile with: tasmc list_max.tasm --group-offset 128
_init:
    importstd mem_8bit

    alias ptr,          mem_8bit::ptrpos
    alias reg,          mem_8bit::cmemreg
    alias largest,      c260
    alias difference,   c261
    alias LIST_START,   30
    alias LIST_END,     39
    alias ACCESS_TIME,  2

    mov c30, 1
    mov c31, 2
    mov c32, 3
    mov c33, 5
    mov c34, 9
    mov c35, 2
    mov c36, 5
    mov c37, 1
    mov c38, 56
    mov c39, 2

    display largest

_start:
    mov largest, c30
    mov ptr, LIST_START
    spawn check_item

; not 0-tickable: waits for the memory controller
check_item:
    spawn mem_8bit::mem         | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait ACCESS_TIME
    ; largest = max(largest, reg) = (largest + reg + |reg - largest|) / 2
    sub difference, reg, largest    | resmode:+
    add largest, reg
    add largest, difference
    div largest, 2
    add ptr, 1
    sle check_item, ptr, LIST_END
```

The last chapter is about what to do when a program doesn't work.
