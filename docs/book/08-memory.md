---
label: 8. Dynamic memory
order: 82
---

# Chapter 8: Dynamic memory
Every instruction you have written so far uses fixed item IDs: `ADD C5, 1` always changes counter 5. That is fine for most programs, but it makes arrays impossible. There is no way to write "the counter whose ID is stored in `C1`".

The standard library's **memory controllers** solve this problem. This chapter shows how to use `mem_8bit` to read and write items by address, and gives a rough idea of how it works.

## 8.1. The memory model
`mem_8bit` treats counters `C1` to `C256` (and timers `T1` to `T256`) as memory. The **address** of a cell is simply its ID: address 10 is `C10`.

The module defines three items for talking to the controller, as aliases:

| Alias | Item | Purpose |
| ----- | ---- | ------- |
| `mem_8bit::ptrpos` | `C257` | The address to read or write. |
| `mem_8bit::cmemreg` | `C258` | The counter value that was read, or that will be written. |
| `mem_8bit::tmemreg` | `T258` | The same, for timers. |

The module also uses `C259` internally. Keep your own variables at `C260` and above, so that they never collide with memory or the controller.

## 8.2. Reading and writing
Every memory operation follows the same three steps:
1. Put the address in `mem_8bit::ptrpos`. For a write, also put the value in `mem_8bit::cmemreg`.
2. Spawn `mem_8bit::mem`, remapping its `mem_end` placeholder to the operation you want:
   - `mem_8bit::cread`: copy the counter at the address into `cmemreg`.
   - `mem_8bit::cwrite`: copy `cmemreg` into the counter at the address.
   - `mem_8bit::tread` and `mem_8bit::twrite`: the same, for timers and `tmemreg`.
3. Wait 2 ticks before using the result or starting another operation.

[`memory_basics.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/memory_basics.tasm) shows all three steps:

```x86asm
; Reads and writes counters by address, using the 8-bit memory controller.
; Compile with a group offset so that the memory controller's remaps
; do not collide with this program's groups:
;   tasmc memory_basics.tasm --group-offset 128
_init:
    import ../stdlib/mem_8bit

    display mem_8bit::ptrpos
    display mem_8bit::cmemreg
    display C10
    display C11

_start:
    ; write 21 to address 10 (counter C10)
    mov mem_8bit::ptrpos, 10
    mov mem_8bit::cmemreg, 21
    spawn mem_8bit::mem     | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cwrite}
    wait 2

    ; read address 10 back into the register and double it
    mov mem_8bit::cmemreg, 0
    spawn mem_8bit::mem     | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait 2
    mul mem_8bit::cmemreg, 2

    ; write the result to the next address (C11)
    add mem_8bit::ptrpos, 1
    wait 2
    spawn mem_8bit::mem     | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cwrite}
```

After it runs, `C10` holds 21 and `C11` holds 42. Of course, for fixed addresses like these, `MOV C10, 21` would be simpler. The point of the controller is that `ptrpos` can be *calculated*, as in the next example.

> [!IMPORTANT]
> Programs that use `mem_8bit` must be compiled with `--group-offset 128` or higher, as the comment at the top of `stdlib/mem_8bit.tasm` explains. The controller remaps IDs 1 to 128 while it runs, and without the offset, it would also redirect your own routines' groups.

## 8.3. A worked example: Fibonacci in memory
[`fib_in_memory.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/fib_in_memory.tasm) fills memory with the Fibonacci sequence, one number per address. Each step reads the two previous numbers, adds them, and writes the sum to the next address:

```x86asm
_init:
	; ensure that stdlib is downloaded and in the same directory as the program
    import ../stdlib/mem_8bit
	alias MAX_INDEX, 50
    alias temp_counter, c260

fib:
    ; read the previous value
    ; all `wait` instructions are necessary for the program to work with `--release`
    spawn mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait 2
    add mem_8bit::ptrpos, 1
    wait 2
    mov temp_counter, mem_8bit::cmemreg	; read from counter register
	wait 2

    spawn mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cread}
    wait 2
    add mem_8bit::cmemreg, temp_counter ; add next number to counter
    wait 2

    ; write the sum into the next memory cell
    add mem_8bit::ptrpos, 1
    wait 2
    spawn mem_8bit::mem | ordered:false remap: {mem_8bit::mem_end = mem_8bit::cwrite}
    wait 1
    
    ; move pointer back to the previous number in preparation for the next iteration
    sub mem_8bit::ptrpos, 1
    
    sl fib, mem_8bit::ptrpos, MAX_INDEX
  
_start:
    mov C2, 1   ; starting value
    mov mem_8bit::ptrpos, 1
    spawn fib
```

Walk through one iteration, starting with `ptrpos` at 1:
1. Read address 1 (`C1` = 0), and save it in `temp_counter` (`C260`, safely above the memory range).
2. Move to address 2 and read it (`C2` = 1). The register now holds 1.
3. Add the saved value: the register holds 0 + 1 = 1.
4. Move to address 3 and write the register there, so `C3` = 1.
5. Move back to address 2, and loop while `ptrpos` is below `MAX_INDEX`.

The next iteration starts at address 2, and so on, until `C1` to `C50` hold the first 50 Fibonacci numbers. Compile it with:

```bash
tasmc fib_in_memory.tasm --group-offset 128
```

[`list_search.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/list_search.tasm) is another example: it walks through a list stored in memory, looking for a number.

## 8.4. How it works
You do not need to understand the controller to use it, but it is a clever piece of TASM.

The final operations are tiny. `cread`, for example, is a single instruction:

```x86asm
cread:  ; write counter at address to memreg
    mov cmemreg, C8200
```

`C8200` is not really the target. It is a placeholder that gets *remapped* to the address. The rest of the controller exists to build that remap:
1. `mem` copies `ptrpos` into a temporary counter, then checks whether it is greater than 128, using an instant compare.
2. If it is, it subtracts 128, then spawns the next check with a remap that shifts IDs up by 128 (IDs 1 to 128 become 129 to 256). Either way, the next check does the same with 64, then 32, and so on, like a binary search.
3. After the last bit, it spawns `mem_end` with `8200` remapped to 1 or 2. Combined with all of the remaps from the earlier steps, `8200` ends up pointing at exactly the address in `ptrpos`.
4. Since you remapped `mem_end` to `cread` (or another operation), that operation runs with `C8200` replaced by the right counter.

This is also why the group offset matters: the remaps in step 2 apply to every kind of ID, including the groups of your own routines.

## 8.5. Larger memory
`mem_14bit` works the same way, but supports addresses from 1 to 16,384. Its registers are at the top of the ID range (`C9997` to `C9999`), and programs that use it must be compiled with `--group-offset 8192` or higher.

> [!NOTE]
> GD only officially supports IDs up to 9,999. Addresses above that rely on remaps reaching IDs that triggers normally cannot use, which is unsupported by GD and may be unstable.

## Summary
- `mem_8bit` lets you read and write `C1`–`C256` and `T1`–`T256` by address.
- Set `ptrpos` (and `cmemreg` for writes), spawn `mem` with `mem_end` remapped to an operation, and wait 2 ticks.
- Compile memory programs with `--group-offset 128`, and keep your own variables at `C260` and above.
- The controller works by composing remaps, one bit at a time.

## Exercises
1. Write a program that fills addresses 1 to 20 with the squares 1, 4, 9, ..., 400.
2. Write a routine that sums the values at addresses 1 to 10 into `C260`.
3. Write a routine that reverses the values at addresses 1 to 10. (Hint: use two pointers, saving them in items above `C260`.)

Next: [Chapter 9: Debugging and common mistakes](09-debugging.md).
