---
label: The TASM Book
icon: book
order: 90
---

# The TASM Book
A hands-on guide to programming Geometry Dash triggers with TASM.

## What this book is
Geometry Dash triggers can count, compare, branch, loop and even address memory. Building programs out of them by hand is slow and fragile: one misplaced trigger and nothing works. TASM lets you write those programs as text, and the compiler, `tasmc`, places the triggers for you.

This book teaches TASM through complete programs. Most of them are based on the programs in [`example_programs/`](https://github.com/ArrowSlashArrow/tasm-lang/tree/master/example_programs), rewritten to follow the [style guide](../style-guide.md). Every chapter builds on the previous one:

| Chapter | You will learn | Programs |
| ------- | -------------- | -------- |
| [1. Getting started](01-getting-started.md) | Installing TASM, compiling, running a program in the level | `counter`, `count_by_five` |
| [2. Items and arithmetic](02-items-and-arithmetic.md) | Counters, timers, arithmetic, aliases, rounding | `temperature`, `clock`, `digits`, `euler_6` |
| [3. Routines and time](03-routines-and-time.md) | Ticks, spawning, waiting, running routines side by side | `two_workers`, `blinker`, `paused_workers` |
| [4. Branching](04-branching.md) | Compares, forks, loops that end, randomness | `countdown`, `collatz`, `is_prime`, `dice` |
| [5. Fast code](05-fast-code.md) | 0-ticking, instant branches, branchless logic | `min_max`, `factorial`, `collatz` (fast), `euler_1`, `random_byte` |
| [6. Routines as functions](06-functions-and-remaps.md) | Calling conventions, passing arguments with remaps | `modulo_remap`, `clock_remap`, `fizz_buzz` |
| [7. Modules and the standard library](07-modules-and-the-stdlib.md) | `import`, `importstd`, writing a library | `shapes`, `stdlib_modulo`, `stdlib_sign`, `big_sum` (`bigint`) |
| [8. Talking to the level](08-the-level.md) | Displays, IOBlocks, timers, collisions, input events, persistence | `stopwatch`, `coin_counter`, `arrows`, `jump_record`, `door` |
| [9. Memory](09-memory.md) | Reading and writing items by address | `memory_basics`, `fib_in_memory`, `list_search` |
| [10. Debugging](10-debugging.md) | Compiler errors, linker output, assertions, common traps | |

## What you need
- Geometry Dash 2.2 and a way to open levels in the editor.
- TASM, installed as described in [chapter 1](01-getting-started.md).
- A text editor. The [VS Code extension](https://github.com/ArrowSlashArrow/tasm-lang/tree/master/vscode-ext) adds syntax highlighting, completion and error checking.

No previous programming experience is required, but it helps. If you have written assembly before, much of TASM will feel familiar.

## How to read this book
Each program is shown with its file name above it:

```nasm hello.tasm
_init:
    display c1

_start:
    mov c1, 42
```

Save it under that name and compile it as described in [chapter 1](01-getting-started.md). Shorter snippets without a file name show a single idea and are not meant to be run on their own.

Every example in this book compiles with the current compiler. Run-time behaviour is described the way the [reference](../index.md) and the [style guide](../style-guide.md) specify it. If a program behaves differently in your level, the [debugging chapter](10-debugging.md) lists the usual causes.

Most chapters end with exercises, followed by their solutions. Try them before reading the solutions.


## Conventions
All code in this book follows the [style guide](../style-guide.md):
- Instructions are lowercase and indented by 4 spaces.
- Flags start in the same column within a routine.
- Rounding modes are written out in full (`floor`, not `f`).
- Values with a meaning get a name with `alias`.
- Routines that can't be 0-ticked say so in a comment (chapter 5 explains what that means).

Instruction names are case-insensitive, so `MOV C1, 2` in older programs means the same as `mov c1, 2`.
