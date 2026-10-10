---
label: The TASM Book
icon: book
order: 90
---

# The TASM Book
An introduction to programming Geometry Dash triggers with TASM.

## Preface
Geometry Dash triggers are surprisingly powerful. With enough Item Edit, Item Compare and Spawn triggers, you can build counters, loops, random number generators, and even memory. Placing those triggers by hand, however, is slow, error-prone, and hard to change later.

TASM is a small assembly-like language that solves this problem. You write a program as text, and the compiler, `tasmc`, turns it into triggers and places them in a level for you. The language stays very close to the triggers that it compiles to, so learning TASM also teaches you how GD's trigger system really behaves.

This book is a guided tour of the language. It starts from installing the compiler and works up to modules, the standard library, and dynamic memory. Every chapter is built around real programs from the [`example_programs`](https://github.com/ArrowSlashArrow/tasm-lang/tree/master/example_programs) folder of the repository, which you can compile and run yourself.

## Who this book is for
You do not need to know assembly to read this book. It helps if you have:
- Used the Geometry Dash editor before, and know what a group and a trigger are.
- Written a little code in any language (variables, `if` statements and loops).

If you are completely new to programming, read slowly and run every example. TASM is a small language, and each chapter only introduces a handful of ideas.

## How to read this book
The chapters are meant to be read in order. Each one ends with a few exercises: try them before moving on, since they cover things that the next chapter assumes you know.

The [reference documentation](../index.md) is the companion to this book. The book explains *why* and *how*; the reference lists every instruction, flag and error in full. When the book mentions an instruction in passing, look it up in the reference for all of its options.

## Contents
1. [Getting started](01-getting-started.md): installing `tasmc`, writing and compiling your first program, and running it in GD.
2. [Thinking in triggers](02-thinking-in-triggers.md): how routines, groups and ticks fit together.
3. [Items and arithmetic](03-items-and-arithmetic.md): counters, timers, math instructions, aliases and flags.
4. [Control flow](04-control-flow.md): spawning, branching, loops, and avoiding race conditions.
5. [Concurrency and timing](05-concurrency.md): routines that run at the same time, waiting, and process control.
6. [Talking to the level](06-the-level.md): displays, IOBlocks, timers, collisions and events.
7. [Modules and the standard library](07-modules.md): splitting programs into files, and using the stdlib.
8. [Dynamic memory](08-memory.md): reading and writing items by address.
9. [Debugging and common mistakes](09-debugging.md): tools, error codes, and pitfalls to watch out for.

## Conventions
Code is written in blocks like this one:
```x86asm
_start:
    MOV C1, 10  ; a comment
```
Instruction names are case-insensitive, so `MOV` and `mov` mean the same thing. The examples in this book use both styles, just like the programs in the repository.

Callouts are used for information that is easy to miss:
> [!NOTE]
> Extra detail that is useful but not essential.

> [!WARNING]
> Behaviour that is likely to surprise you, including known compiler bugs.

This book describes TASM **v0.3.2**.
