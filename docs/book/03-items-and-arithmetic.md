---
label: 3. Items and arithmetic
order: 87
---

# Chapter 3: Items and arithmetic
This chapter covers how to store and calculate values. By the end of it, you will be able to read and write programs like [`temperature.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/temperature.tasm), which converts temperatures between units.

## 3.1. Counters and timers
As Chapter 2 explained, all data lives in **items**. You write an item as a letter followed by its ID:

| Item | Written as | Holds | Range |
| ---- | ---------- | ----- | ----- |
| Counter | `C1` to `C9999` | Whole numbers | -2,147,483,648 to 2,147,483,647 |
| Timer | `T1` to `T9999` | Decimal numbers (32-bit floats) | Up to 9,999,999 when set by arithmetic |

The letter is case-insensitive, so `c5` and `C5` are the same counter. All items start at 0.

Use counters for whole numbers, such as loop counters and scores. Use timers when you need decimals, such as for angles or averages.

> [!NOTE]
> Timers are 32-bit floats, so they cannot represent every whole number above 16,777,216 exactly. Large values may be slightly rounded by GD.

There are also three built-in items:
- `POINTS`: the level's points counter. You may read and write it.
- `ATTEMPTS`: the attempt counter. Read-only.
- `MAINTIME`: the level's main timer. Read-only.

These names must be written in uppercase. Writing to `ATTEMPTS` or `MAINTIME` is a compile error (E0004).

## 3.2. Arithmetic instructions
TASM has six basic arithmetic instructions, and each one compiles to a single Item Edit trigger that takes one tick:

| Instruction | Meaning |
| ----------- | ------- |
| `MOV` | Assign a value |
| `ADD` | Add |
| `SUB` | Subtract |
| `MUL` | Multiply |
| `DIV` | Divide |
| `FLDIV` | Divide, then round down (floor) |

The first argument is always where the result goes. The other arguments can be written in a few different forms:

```x86asm
example:
    MOV C1, 10          ; C1 = 10
    MOV C2, C1          ; C2 = C1
    ADD C1, 5           ; C1 = C1 + 5
    ADD C1, C2          ; C1 = C1 + C2
    ADD C3, C1, C2      ; C3 = C1 + C2
    MUL C3, C1, 4       ; C3 = C1 * 4
```

In short:
- **Two arguments** (`ADD C1, 5` or `ADD C1, C2`) update the first item in place, like `+=` in other languages.
- **Three items** (`ADD C3, C1, C2`) store the result of the second and third item in the first.
- **Two items and a number** (`MUL C3, C1, 4`) stores the second item times the number in the first. Only `MUL`, `DIV` and `FLDIV` accept this form.

`MOV` only accepts the two-argument forms.

> [!WARNING]
> In the v0.3.2 compiler, the "two items and a number" form of `DIV` and `FLDIV` multiplies instead of dividing: `DIV C1, C2, 4` sets `C1` to `C2 * 4`. Until this is fixed, either copy the value first and divide in place:
> ```x86asm
>     MOV C1, C2
>     DIV C1, 4
> ```
> or add the `divmod:true` flag (flags are explained below): `DIV C1, C2, 4 | divmod:true`.

Dividing by zero does not crash anything. GD simply sets the result to 0.

### Modulo
There is no modulo instruction, because there is no trigger that calculates one. Instead, you calculate it from the other operations: `a mod b = a - floor(a / b) * b`. The Project Euler example [`project_euler_1.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/project_euler_1.tasm) does this to check whether `C1` is divisible by 3:

```x86asm
add_if_mod_3:
    MOV C3, C1      ; clone to temp var
    FLDIV C3, 3     ; C3 = floor(C1 / 3)
    MUL C3, 3       ; C3 = floor(C1 / 3) * 3
    SUB C3, C1, C3  ; C3 = C1 - floor(C1 / 3) * 3 = C1 mod 3
```

The standard library also has a ready-made `math::modulo` routine, which you will use in Chapter 7.

## 3.3. Aliases
Programs full of `C1`, `C2` and `T3` get hard to read quickly. The `ALIAS` instruction gives a value a name. It may only be used in `_init`:

```x86asm
_init:
    ALIAS score, C1
    ALIAS BONUS, 50

_start:
    ADD score, BONUS    ; same as ADD C1, 50
```

An alias is replaced by its value before anything else happens, so you can use it anywhere you would write the value itself. Aliases can name items, numbers, groups, or anything else. A few rules:
- Aliases are constants. You cannot define the same name twice (E0013).
- An alias name must not look like another value. Names such as `c5`, `g2` or `42` are rejected (E0015).
- An alias cannot refer to another alias. `ALIAS b, a` makes `b` the text `a`, not the value of `a`.

A common style, used throughout the repository, is lowercase names for items and uppercase names for constants.

## 3.4. A worked example: temperature conversion
Here is [`temperature.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/temperature.tasm) in full:

```x86asm
; Converts a temperature from Celsius to Fahrenheit and Kelvin.
; Timers (T) can hold decimals; counters (C) only hold whole numbers.
_init:
    alias celsius, T1
    alias fahrenheit, T2
    alias kelvin, T3
    alias rounded_f, C1
    alias START_TEMP, 21.5

    display celsius
    display fahrenheit
    display kelvin
    display rounded_f

_start:
    mov celsius, START_TEMP

    ; fahrenheit = celsius * 1.8 + 32
    mul fahrenheit, celsius, 1.8
    add fahrenheit, 32

    ; kelvin = celsius + 273.15
    mov kelvin, celsius
    add kelvin, 273.15

    ; store fahrenheit as a whole number, rounded to the nearest integer
    mov rounded_f, fahrenheit   | resmode:round
```

Most of this should look familiar by now:
- Timers are used for the temperatures, since they have decimals.
- `ADD` has no "two items and a number" form, so Kelvin is calculated by copying `celsius` first, then adding to the copy.

The last line introduces something new: a **flag**.

## 3.5. Flags
Item Edit triggers have far more options than the basic instructions use. **Flags** give you access to those options. They are written at the end of the line, after a `|`:

```x86asm
    mov rounded_f, fahrenheit   | resmode:round
```

`resmode` sets how the result is rounded. Here, 70.7 is rounded to 71 before it is stored in the counter. Without it, you have no control over how the decimal is turned into a whole number.

Flags follow strict rules:
- A flag is written as `name:value`, with **no space** after the colon.
- Several flags are separated by spaces, all after a single `|`.
- Flag names are lowercase.

```x86asm
    ADD C1, C2 | itemmod:0.5 resmode:floor   ; correct
    ADD C1, C2 | itemmod: 0.5                ; error: space after the colon
    ADD C1, C2 | itemmod:0.5 | resmode:floor ; error: two pipes (E0021)
```

The flags you will use most with arithmetic are:

| Flag | What it does | Example |
| ---- | ------------ | ------- |
| `resmode` | Rounds and/or changes the sign of the result. Rounding is `round`, `floor` or `ceil` (or `r`, `f`, `c`). Sign is `+` (absolute value) or `-` (negative absolute value). They can be combined, such as `floor+`. | `MOV C1, C1 \| resmode:+` makes `C1` positive. |
| `itemmod` | Multiplies the item part of the calculation by a number. | `ADD C1, C2 \| itemmod:0.5` adds half of `C2` to `C1`. |
| `divmod` | Makes `itemmod` divide instead of multiply. | `ADD C1, C2 \| itemmod:4 divmod:true` adds a quarter of `C2`. |

Flags can replace whole routines. Before flags existed, taking the absolute value of a number required a comparison and a second routine. Now, it is a single instruction:

```x86asm
absolute:
    MOV C1, C1 | resmode:+
```

> [!NOTE]
> An instruction silently ignores flags that it does not use. If a flag seems to have no effect, check the instruction's **Flags** list in the [reference](../index.md#312-available-instructions). Flag values must be written out: aliases do not work inside flags like `itemmod`.

## Summary
- Counters hold whole numbers, timers hold decimals. Both are global.
- `MOV`, `ADD`, `SUB`, `MUL`, `DIV` and `FLDIV` each compile to one Item Edit trigger and take one tick.
- `ALIAS` gives values readable names.
- Flags such as `resmode` and `itemmod` unlock more of the Item Edit trigger's options.

## Exercises
1. Extend `temperature.tasm` to convert Fahrenheit back to Celsius: `celsius = (fahrenheit - 32) / 1.8`. Store the result in `T4`.
2. Write a routine that sets `C3` to the average of `C1` and `C2`, rounded down. Try to do it in two instructions. (Hint: `ADD C3, C1, C2`, then use a flag.)
3. Using the modulo technique from section 3.2, calculate `C1 mod 7` into `C2`.

Next: [Chapter 4: Control flow](04-control-flow.md).
