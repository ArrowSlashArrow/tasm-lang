---
label: 1. Getting started
order: 89
---

# Chapter 1: Getting started
In this chapter you will install TASM, write a program that counts upwards, put it into a level, and run it.

## 1.1. Installing TASM
TASM comes with install scripts for Windows and Linux. Download the script for your system from the `install/` folder of the [repository](https://github.com/ArrowSlashArrow/tasm-lang/tree/master/install) and run it with the version you want:

```powershell
# Windows (PowerShell)
.\windows.ps1 install v0.3.2
```
```bash
# Linux
./linux.sh install v0.3.2
```

The script asks which build to install. On Windows, pick MSVC unless it doesn't work for you; on Linux, either build is fine. It installs everything into one folder (`%APPDATA%\tasm` on Windows, `~/.local/bin/tasm` on Linux) and adds that folder to your `PATH`:
- `tasmc`: the compiler.
- `stdlib/`: the standard library, used in [chapter 7](07-modules-and-the-stdlib.md).
- `tasm-lsp`: the language server used by the VS Code extension (newer releases only).

Restart your terminal, then check that the compiler runs:

```bash
tasmc --version
```

To build the compiler from source instead, install Rust and run `cargo build --release` in the `rtasm/` folder of the repository. The executable ends up in `rtasm/target/release/`.

## 1.2. Your first program
Create a file called `counter.tasm`:

```nasm counter.tasm
; Counts upwards forever.
_init:
    display c1

_start:
    add c1, 1
    spawn _start
```

Line by line:
- `; Counts upwards forever.` is a **comment**. Everything after a `;` is ignored by the compiler.
- `_init:` starts the **`_init` routine**. It holds setup instructions that apply to the whole level. Here, `display c1` places a counter display on the screen that shows the value of **counter 1**, written `c1`.
- `_start:` starts the **`_start` routine**, where the program begins.
- `add c1, 1` adds 1 to counter 1.
- `spawn _start` starts `_start` again, so the program keeps counting.

A **routine** is a list of instructions under a name ending in `:`. The name must start at the beginning of the line, and every instruction under it must be indented. This book uses 4 spaces, as the style guide recommends.

## 1.3. Compiling and running
Close Geometry Dash first: the game overwrites its save file when it exits, so a level added while it is open would be lost. Then compile the program:

```bash
tasmc counter.tasm
```

The compiler prints what it is doing:

```text
Parsing tasm...
Encoding level...
Using groups 1 - 1
Exported to savefile.
```

The program has been added to your save file as a new level called `counter.tasm`. Start Geometry Dash, open the level in the editor, and you will see:
- a counter display to the left of the start, showing `c1`.
- a block labelled **start**. Touching it with the player runs `_start`.
- the program's triggers, in a row labelled `1: _start`.

Start playtesting, walk into the **start** block (compiled levels use platformer mode), and watch the counter go up.

> [!TIP]
> Other ways to export a program:
> - `tasmc counter.tasm --gmd` writes a `.gmd` file that you can import with a level-sharing mod, instead of touching the save file.
> - `tasmc counter.tasm --clipboard` copies the objects to the clipboard, so you can paste them into an open level with BetterEdit (Windows only).
> - `tasmc counter.tasm --no-export` only checks the program for errors.

## 1.4. What the compiler made
Each instruction became a trigger:
- `add c1, 1` is an **Item Edit** trigger that adds 1 to counter 1.
- `spawn _start` is a **Spawn** trigger that spawns the routine's own group.

Every routine (except `_init`) is compiled into one **group**. `Using groups 1 - 1` means the program used group 1, for `_start`. The **start** block is an **IOBlock**: a block with a touch-activated spawn trigger, which the compiler always adds for `_start`.

The triggers of a routine are laid out from left to right and run in that order, one after another. How long that takes depends on how you compile:
- By default, the compiler places triggers 30 units (one grid block) apart. That is easy to read in the editor, but each instruction then takes 30 ticks, or 1/8 of a second.
- With `--release` (or `-r`), the triggers are packed 1 unit apart, and each instruction takes a single **tick**. GD runs 240 ticks per second.

Compile `counter.tasm` again with `-r` and the counter goes up about 30 times faster. Use the default mode while writing a program, so that you can see what is happening, and release mode once it works.

## 1.5. When something goes wrong
Change `add` to `ad` and compile again:

```text
Parsing tasm...
[E0044] Unable to compile "...\counter.tasm" with 1 errors:
...\counter.tasm @ _start:5 [E0025] InvalidInstruction: Unrecognized instruction AD
Unable to parse main module: lexing failed
```

The error names the routine (`_start`), the line (5) and an error code (`E0025`). For a longer explanation of any error code, run `tasmc -e` with its number:

```bash
tasmc -e 25
```

Nothing is exported when there are errors. [Chapter 10](10-debugging.md) covers the most common errors and how to fix them.

> [!TIP]
> The VS Code extension shows these errors while you type, so you rarely need to compile just to find them.

## 1.6. Exercises
1. Make the counter go up by 5 each time.
2. Add a second display for `c2`, and make `c2` count down by 1 at the same time as `c1` counts up.

Solution 1
```nasm count_by_five.tasm
_init:
    display c1

_start:
    add c1, 5
    spawn _start
```


Solution 2
```nasm up_and_down.tasm
_init:
    display c1
    display c2

_start:
    add c1, 1
    sub c2, 1
    spawn _start
```
Both changes happen in the same routine, so `c1` and `c2` change one instruction apart.


In the next chapter you will work with numbers in more detail: the different kinds of items, every arithmetic instruction, and how to give values readable names.
