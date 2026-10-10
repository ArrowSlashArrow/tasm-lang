---
label: 9. Debugging
order: 81
---

# Chapter 9: Debugging and common mistakes
Trigger programs fail in unusual ways. A typo might compile into a perfectly valid program that does the wrong thing, and a timing problem might only appear in release mode. This last chapter collects tools and habits that help, and the mistakes that nearly everyone makes at least once.

## 9.1. Compiler tools
`tasmc` has several options that help you understand what it is doing:

| Option | Use it to |
| ------ | --------- |
| `--no-export` | Check a program for errors without touching your savefile. |
| `-e <code>`, `--error-help <code>` | Get a longer explanation of an error code: `tasmc -e 27` explains E0027. |
| `-l`, `--linker-output` | Print every routine after linking, with its group, instructions and flags. This shows exactly what your aliases and imports turned into. |
| `-D`, `--dependencies` | List every module that the program imports, directly or indirectly. |
| `-v`, `--verbose-logs` | Print more detail while compiling. |

For example, `--linker-output` on [`collatz.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/collatz.tasm) shows the aliases replaced by real items:

```
step: (2)
    FE Group(3), Group(4), Counter(1), Counter(1) | Flag { ident: "lmod", ... }
even: (3)
    DIV Counter(1), Number(2.0) |
    SPAWN Group(5) |
```

If a value shows up with the wrong type here (for example, `String` where you expected a `Counter`), you have found your bug.

## 9.2. Watching a program run
In the level itself:
- **Compile in debug mode** (the default) while you are testing. Each instruction takes 30 ticks, which is slow enough to follow by eye.
- **`DISPLAY` everything.** Show every item that your program uses, including temporary ones.
- **Add IOBlocks for each step.** Instead of running a whole program from `start`, add IOBlocks that spawn individual routines, so that you can test them one at a time.
- **Use assertions.** The standard library's `debug` module flashes the background green or red (see Chapter 7), which is useful for checking results automatically.

## 9.3. Reading error messages
Errors include the file, the routine and the line number:

```
...\example.tasm @ _start:5 [E0027] InvalidInstruction: Instruction MOV has no argument handler for the argset [Item, String]
```

The compiler reports every error it finds, followed by an E0044 error that says how many there were. Start with the first one. This error says that `MOV` was given an item and a string, and that no form of `MOV` accepts those. A `String` in an error message usually means that a name was misspelled: the compiler could not find a routine or alias with that name, so it treated it as text.

These are the errors that you are most likely to see:

| Error | Usual cause |
| ----- | ----------- |
| E0011 | No `_start` routine. Add one, or compile a library with `--no-entry-point`. |
| E0013 | The same alias is defined twice. |
| E0015 | An alias name looks like another value, such as `c5` or `g2`. |
| E0021 | More than one `\|` on a line. Put all flags after a single `\|`. |
| E0023 | `ALIAS` outside of `_init`. |
| E0025 | Misspelled instruction name. |
| E0027 | Wrong arguments for an instruction. Check the types in the message against the reference. |
| E0028 | Two routines with the same name. |
| E0029 | An unindented line that is not a routine label. Indent the instructions. |
| E0030 | Usually a missing space in `remap: {module::name = ...}`. |
| E0031 | Unknown flag. Usually caused by uppercase letters in a flag name, or a space after the colon (`itemmod: 0.5`). |
| E0032 | A flag value that could not be read, such as an alias in `itemmod`. |
| E0034 | `IMPORT` outside of `_init`. |
| E0043 | An import path that does not exist. Paths are relative to the importing file, without `.tasm`. |
| E0045 | Two modules import each other. |

## 9.4. Common mistakes
### Forgetting that spawns do not wait
The most common bug in TASM is reading a result before the routine that produces it has finished. If a value is sometimes right and sometimes wrong, or right in debug mode and wrong in release mode, look for a race condition. Use the "branch last, then continue" pattern from [Chapter 4](04-control-flow.md#45-race-conditions), or wait long enough.

### Testing only in debug mode
Debug mode and release mode have the same instruction order, but different timings: spawn delays and timers take the same real time in both, while instructions are 30 times slower in debug mode. A program whose timing works in one mode may break in the other. Test the final program with `--release`.

### Naming routines like values
A routine called `c5`, `g12` or `inf` is read as a counter, a group or a number when you refer to it, not as your routine. Choose names that cannot be mistaken for anything else.

### Spaces in flags
`itemmod: 0.5` is an error, but `remap: {...}` needs the space when it uses `::`. When in doubt: no space after the colon, except before a `{`.

### Remapping more than you meant to
A remap changes every kind of ID, so `remap: {1 = 3}` affects counter 1, timer 1 *and* group 1. See [Chapter 7](07-modules.md#75-passing-arguments-with-remaps).

### Waits in unordered routines
`WAIT`, `NOP` and `WAITS` do nothing in a routine that was spawned with `ordered:false`, or from an instant compare.

### Running out of groups
Every routine uses a group, and every base compare uses one or two more. If you are close to the limit, use instant compares where timing allows, and reuse routines instead of copying them.

## 9.5. Known compiler issues
TASM is still young, and v0.3.2 has a few bugs that are worth knowing about:
- `IMPORTSTD` cannot find the standard library (see [Chapter 7](07-modules.md#76-importstd)).

The [reference](../index.md) documents each of these in detail, along with workarounds.

## 9.6. Where to go next
You now know every major part of TASM. From here:
- Keep the [reference documentation](../index.md) open while you write programs. It documents every instruction, flag and option.
- Read the standard library in [`stdlib`](https://github.com/ArrowSlashArrow/tasm-lang/tree/master/stdlib). It is full of tricks that are not covered here, such as branchless comparisons in `compare.tasm` and single-instruction logic in `bits.tasm`.
- Try the [Project Euler](https://projecteuler.net/) problems. The repository already has solutions to problems [1](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/project_euler_1.tasm), [2](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/project_euler_2.tasm) and [6](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/project_euler_6.tasm). Many of the others are good practice.
- Contribute! Bug reports, new stdlib modules and example programs are all welcome on the [TASM repository](https://github.com/ArrowSlashArrow/tasm-lang).

Happy triggering.
