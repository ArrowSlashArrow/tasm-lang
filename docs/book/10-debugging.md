---
label: 10. Debugging
order: 80
---

# Chapter 10: Debugging
Programs go wrong in two ways: the compiler rejects them, or they compile and then do the wrong thing in the level. This chapter covers both. It lists the errors you are most likely to see, the tools for looking inside a program, and the traps behind most programs that compile but misbehave.

## 10.1. Reading compiler errors
An error message has the same parts every time:

```text
...\bad_flag.tasm @ _start:2 [E0031] BadFlag: Unrecognized flag orderd:false
```

- the file, which matters once a program imports modules.
- the routine and the line number, counted from the top of the file.
- the error code, `E0031`. `tasmc -e 31` prints a longer explanation.
- the kind of error and a description.

The compiler checks a program in two stages. **Parsing** (`Parsing tasm...`) reads every line and resolves names. **Encoding** (`Encoding level...`) places the triggers. If parsing fails, the compiler lists every error it found in one go. Encoding stops at the first error.

## 10.2. Common errors
**A misspelled routine:**

```nasm misspelled_routine.tasm does not compile
_start:
    spawn finsh

finish:
    mov c1, 1
```
```text
... @ _start:2 [E0027] InvalidInstruction: Instruction SPAWN has no argument handler for the argset [String]
```

A name that isn't a routine is read as a string, and no instruction takes a string where it expects a routine. **E0027** always means "these arguments don't fit this instruction". The argset lists how the compiler read each argument, so compare it with the instruction's arguments in the [reference](../index.md):

```text
... @ _start:2 [E0027] InvalidInstruction: Instruction ADD has no argument handler for the argset [Number, Item]
```

Here `add 5, c1` has its arguments the wrong way round: the result must come first.

**A misspelled flag** gives E0031, as above. Flags are checked by name, so this catches typos.

**An `_init`-only instruction elsewhere:**

```text
... @ _start:2 [E0026] InvalidInstruction: Instruction DISPLAY is not allowed in routine _start because it is exclusive to the initialiser routine, _init.
```

`display`, `ioblock`, `event`, `alias`, `import` and `importstd` belong in `_init`.

**An alias that looks like a value:**

```text
... @ _init:2 [E0015] InvalidInstruction: Bad alias identifier: c5
```

`alias c5, 10` would make `c5` mean two things. Alias names can't look like items, groups, numbers or built-in names.

**No `_start`:**

```text
... [E0011] NoEntryPoint: No entry point found in file.
```

Add a `_start` routine, or compile with `--no-entry-point` if the file is a module.

**A missing module:**

```text
Unable to parse main module: [E0043] Unable to resolve path for module C:\Users\...\AppData\Roaming\tasm\stdlib\consts in ...\shapes.tasm: The system cannot find the path specified. (os error 3)
```

The path is where the compiler looked. For `importstd`, that is the `stdlib` folder in the installation; see [chapter 7](07-modules-and-the-stdlib.md#72-the-standard-library).

**An error while encoding:**

```nasm wrong_collision_order.tasm does not compile
_start:
    coll hit, b1, COLL_P1

hit:
    mov c1, 1
```
```text
Encoding level...
Unable to compile to level with 1 errors:
... @ _start:2 [E0048] InvalidInstruction: Cannot detect collision between CollBlock(1) and Player1
```

The player must come first: `coll hit, COLL_P1, b1`. Other encoding errors are about limits, such as using more than 9,999 groups (E0001).

## 10.3. Looking inside a program
**Displays.** The simplest tool: `display` every item you are unsure about. Compile in debug mode, where each instruction takes 1/8 of a second, and watch the values change one step at a time.

**Linker output.** `tasmc program.tasm --no-export -l` prints every routine with its group, and every instruction with its arguments as the compiler understood them, after aliases and module names have been resolved:

```nasm double.tasm
_init:
    alias n,            c1

_start:
    mov n, 5
    spawn double                | ordered:false delay:0

; @0tick
double:
    mul n, 2
```
```text
------- linker output -------
_init: (0)
_start: (1)
    MOV Counter(1), Number(5.0) |
    SPAWN Group(2) | Flag { ident: "ordered", value: Bool(false), _ftype: Bool }, Flag { ident: "delay", value: Float(0.0), _ftype: Float }
double: (2)
    MUL Counter(1), Number(2.0) |
----- end linker output -----
```

Use it to check that an alias means what you think, that a module routine was found, and which groups your routines use, for example to make sure no remap hits them ([chapter 6](06-functions-and-remaps.md#63-remaps-change-every-kind-of-id)).

**Assertions.** `debug::assert_eq_int` flashes the background green if `c1` equals `c2` and red otherwise, and `debug::assert_eq_float` does the same for `t1` and `t2`. [Chapter 7](07-modules-and-the-stdlib.md#73-example-testing-with-debug) shows them in use. They make good tests: run a routine on a known input and check the answer.

**Buttons.** An IOBlock that runs one routine lets you test it on its own, and IOBlocks that `pause` and `resume` a loop let you stop it to look at the displays ([chapter 3](03-routines-and-time.md#35-pausing-resuming-and-stopping)).

**The editor.** Each routine is a row of triggers labelled with its group and name. Clicking a trigger shows exactly what it does, which helps when a flag doesn't do what you expected.

## 10.4. Programs that compile but misbehave
| Symptom | Likely cause |
| ------- | ------------ |
| A loop runs once and stops. | The routine spawned itself while it was still running, e.g. with a 0-ticked spawn or an instant branch. Use a normal spawn, or a compare with `ordered:false` ([chapter 5](05-fast-code.md#54-what-cant-be-0-ticked)). |
| A wait does nothing. | The routine was started with `ordered:false` or by an instant branch, so it isn't spawn ordered. |
| A value is read before it is ready. | The routine that computes it wasn't 0-ticked, so it finishes later than the next instruction. 0-tick it, or wait, as with memory accesses ([chapter 9](09-memory.md)). |
| A routine works on its own, but not when called with a remap. | The remap hits one of the program's groups. Move the parameters to high IDs ([chapter 6](06-functions-and-remaps.md#63-remaps-change-every-kind-of-id)). |
| A branch runs the wrong routine. | The target routine is empty; give it a `nop` ([chapter 4](04-branching.md#47-branches-that-do-nothing)). |
| A comparison with a number ignores the number. | `rmod` replaces the number on the right side ([chapter 4](04-branching.md#44-comparing-modified-values)). |
| Large constants come out slightly wrong. | Constants in instructions are 32-bit floats, so above 16,777,216 they may be rounded ([chapter 2](02-items-and-arithmetic.md)). |
| Two routines that run at the same time give different results each run. | A race condition: both read or write the same item on the same tick. Give each its own items, or make one wait for the other. |
| `div` or `fldiv` with three arguments, the last one a number, multiplies. | A bug in TASM v0.3.2, fixed in later versions. Use `mov x, a` followed by `div x, number` instead. |
| Nothing happens when the **start** block is touched. | In debug mode, the first changes may take a while. Check the displays, and check that the level was saved before GD was closed ([chapter 1](01-getting-started.md#13-compiling-and-running)). |

When none of these fit, make the program smaller: remove routines until the problem goes away, and look closely at the last thing you removed.

## 10.5. Where to go from here
You now know all of TASM. To keep going:
- Read the [reference](../index.md) for every instruction and flag.
- Read the [style guide](../style-guide.md) again; most of it should now feel familiar.
- Read [`example_programs/bigint.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/bigint.tasm) and the [standard library](https://github.com/ArrowSlashArrow/tasm-lang/tree/master/stdlib) to see larger programs.
- Write your own module, and consider contributing it ([CONTRIBUTING.md](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/CONTRIBUTING.md)).
