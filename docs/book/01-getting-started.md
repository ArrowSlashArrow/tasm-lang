---
label: 1. Getting started
order: 89
---

# Chapter 1: Getting started
In this chapter, you will install the TASM compiler, compile a small program, and watch it run inside Geometry Dash.

## 1.1. Installing the compiler
The TASM compiler is called `tasmc`. The easiest way to get it is with an install script from the repository's `install/` folder:

```powershell
# Windows (PowerShell)
.\windows.ps1 install v0.3.2
```
```bash
# Linux
./linux.sh install v0.3.2
```

The script asks which build you want (pick MSVC on Windows and glibc on Linux if you are unsure), downloads it, and adds it to your `PATH`. Restart your terminal afterwards, then check that it works:

```bash
tasmc --version
```

If you would rather build from source, install Rust and run `cargo build --release` in the `rtasm` folder. The executable ends up in `rtasm/target/release/`. See [Installing TASM](../index.md#41-installing-tasm) in the reference for all options.

## 1.2. Your first program
Create a file called `incrementer.tasm` with the following contents (it is also in [`example_programs/incrementer.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/incrementer.tasm)):

```x86asm
_init:
    DISPLAY C234

_start:
    ADD C234, 1
    SPAWN _start
```

Even this tiny program shows the basic shape of every TASM file:
- A program is made of **routines**. A routine starts with a name followed by a colon (`_start:`), on a line with no indentation.
- Each indented line under a routine is an **instruction**. An instruction is a name (`ADD`) followed by comma-separated arguments (`C234, 1`).
- `C234` is a **counter**: counter item number 234 in GD. Counters hold whole numbers.

Two routine names are special:
- `_init` runs once when the level starts. It is used for setup, such as `DISPLAY C234`, which places a counter label in the level so that you can see the value of `C234`.
- `_start` is the entry point of the program, much like `main` in other languages.

So the program reads: "show counter 234 on screen; then, when the program starts, add 1 to counter 234 and start again". It counts up forever.

## 1.3. Compiling
Close Geometry Dash, then run:

```bash
tasmc incrementer.tasm
```

The compiler prints something like:
```
Parsing tasm...
Encoding level...
Using groups 1 - 1
```

By default, `tasmc` adds the program to your GD savefile as a new level named after the file. This is why GD should be closed: the game rewrites the savefile when it exits, and would throw away the new level.

> [!NOTE]
> If the compiler cannot find your savefile, or you want to share the level, export it as a `.gmd` file instead with `tasmc incrementer.tasm --gmd`. If you just want to check a program for errors, use `--no-export`.

## 1.4. Running it
Open GD and find the new level, `incrementer.tasm`, in your created levels. Open it in the editor. You will see:
- A counter label on the left, showing the value of `C234`.
- A row of triggers on the right. These are your routines.
- A block labelled `start` at the bottom. This is an **IOBlock**: when the player touches it, it spawns `_start`.

Compiled levels are platformer levels. Playtest the level, walk into the `start` block, and the counter starts counting up.

## 1.5. Debug mode and release mode
If you watched closely, the counter went up quite slowly: only a few times per second. That is because the compiler is in its default **debug mode**, which spaces instructions 30 units apart so that you can read them in the editor. GD runs the triggers in a routine from left to right, so this spacing also makes the program slow.

Compile again with `--release` (or `-r`):

```bash
tasmc incrementer.tasm --release
```

In release mode, instructions are packed 1 unit apart, and the program runs at full speed: one instruction per tick, and GD runs 240 ticks per second. Use debug mode while you are learning or looking for a bug, since you can watch your program step by step. Use release mode when the program is finished.

## 1.6. Comments
Anything after a `;` is a comment, and is ignored by the compiler:

```x86asm
_start:
    ADD C234, 1     ; count up
    SPAWN _start    ; and do it again
```

Use comments generously. Trigger programs are hard to read without them.

## Exercises
1. Change `incrementer.tasm` so that it counts up by 5 instead of 1.
2. Add a second counter, `C1`, that counts *down* (hint: use `SUB`). Remember to `DISPLAY` it.
3. Compile the program with `--gmd` and find the `.gmd` file that it creates.

Next: [Chapter 2: Thinking in triggers](02-thinking-in-triggers.md).
