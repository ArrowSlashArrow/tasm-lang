---
label: 2. Items and arithmetic
order: 88
---

# Chapter 2: Items and arithmetic
Programs store their data in GD's **items**. In this chapter you will learn which items there are, how to do arithmetic on them, how to give them readable names, and how to control rounding.

## 2.1. Items
An item holds a single number. There are two kinds you can use freely:

| Kind | Written | Holds |
| ---- | ------- | ----- |
| Counter | `c1` to `c9999` | Whole numbers (32-bit, from about -2.1 billion to 2.1 billion) |
| Timer | `t1` to `t9999` | Decimal numbers (item edits can't set them above 9,999,999) |

Counter 1 and timer 1 are different items. The letter can be upper- or lowercase: `C5` and `c5` are the same counter.

GD also has three built-in items, written in uppercase:
- `POINTS`: the level's points counter. You can read and write it.
- `ATTEMPTS`: the attempt number. Read-only.
- `MAINTIME`: the time since the level started, in seconds. Read-only.

Writing to `ATTEMPTS` or `MAINTIME` is a compile error (E0004).

> [!NOTE]
> Arithmetic on items is exact. Only the constant in an instruction (the `5` in `add c1, 5`) is stored as a 32-bit float, so constants above 16,777,216 (2<sup>24</sup>) may be rounded: `mov c1, 16777217` sets `c1` to 16777216. Build bigger values from smaller constants instead, e.g. `mov c1, 16777216` then `add c1, 1`.

## 2.2. Arithmetic instructions
| Instruction | Does |
| ----------- | ---- |
| `mov` | assigns |
| `add` | adds |
| `sub` | subtracts |
| `mul` | multiplies |
| `div` | divides |
| `fldiv` | divides and rounds the result down ("floor division") |

Each of them takes the item to write to first. They accept these forms:

```nasm arithmetic.tasm
_init:
    display c1
    display c2
    display c3

_start:
    mov c1, 10          ; c1 = 10
    mov c2, c1          ; c2 = c1           (10)
    add c1, 5           ; c1 = c1 + 5       (15)
    sub c1, c2          ; c1 = c1 - c2      (5)
    mul c3, c1, c2      ; c3 = c1 * c2      (50)
    div c3, c1          ; c3 = c3 / c1      (10)
    sub c3, c3, c1      ; c3 = c3 - c1      (5)
    mul c2, c1, 3       ; c2 = c1 * 3       (15)
    fldiv c2, 4         ; c2 = floor(c2 / 4) (3)
```

In short:
- `op a, number` and `op a, b` update `a` in place.
- `op a, b, c` stores `b op c` in `a`. `mov` doesn't have this form.
- `mul a, b, number` stores `b * number` in `a`.

> [!WARNING]
> `div a, b, number` and `fldiv a, b, number` also exist, but compile to a *multiplication* in TASM v0.3.2 and older. This book avoids them: to divide one item by a number into another item, copy it first.
> ```nasm
> half:
>     mov c2, c1
>     div c2, 2
> ```

Dividing by zero doesn't stop the program: GD sets the result to 0.

## 2.3. Naming values with aliases
`c3` doesn't tell you what the counter is for. An **alias** gives a value a name. Aliases are defined in `_init`:

```nasm interest.tasm
; Adds 5% interest to a balance, three times.
_init:
    alias balance,      t1
    alias RATE,         1.05

    display balance

_start:
    mov balance, 200
    mul balance, RATE
    mul balance, RATE
    mul balance, RATE   ; 231.525
```

When the compiler sees `balance`, it replaces it with `t1`; `RATE` becomes `1.05`. This book writes aliases for items in lowercase and aliases for constant numbers in uppercase, so the two are easy to tell apart.

The style guide asks you to name every **magic value**: a number or ID whose meaning isn't obvious from the code. A few rules:
- An alias replaces a whole argument only. In `add balance, 1`, `balance` is replaced; in `add balance2, 1`, nothing happens.
- An alias can't be named like another value. `c5`, `g2`, `42` and `POINTS` are rejected as names (E0015).
- An alias can't be changed once it is defined (E0013).
- Aliases don't work inside most flags (section 2.6). `| itemmod:RATE` is an error.

## 2.4. Counters and timers together: temperature
This program converts a temperature from Celsius to Fahrenheit and Kelvin. It is [`example_programs/temperature.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/temperature.tasm):

```nasm temperature.tasm
; Converts a temperature from Celsius to Fahrenheit and Kelvin.
; Timers (T) can hold decimals; counters (C) only hold whole numbers.
_init:
    alias celsius,      t1
    alias fahrenheit,   t2
    alias kelvin,       t3
    alias rounded_f,    c1
    alias START_TEMP,   21.5

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

21.5 °C is 70.7 °F and 294.65 K, so the timers show `70.70` and `294.65`, and `rounded_f` shows `71`.

The temperatures are timers because they have decimals. The last line stores the Fahrenheit value in a counter, which can only hold whole numbers. The `| resmode:round` part is a **flag**: it rounds the value to the nearest whole number before it is stored. Without it, you leave it to GD to drop the fraction.

> [!NOTE] Be careful with large or precise numbers.
> GD will not correctly load large and/or precise numbers directly. Keep this in mind when using precise values, such as those from `stdlib/consts.tasm`.  
> See [section 3.1.2.1.1 of the documentation](../index.md#31211-caveats-with-using-largeprecise-numbers) for more info.

## 2.5. Rounding
Rounding is controlled with two flags:
- `resmode` rounds the **item result**: the value computed from the input items, before it is combined with the target. For `mov` and the three-item forms, that is the value that gets stored.
- `finmode` rounds the **final result**, after it has been combined with the target. It only applies when the instruction updates the target in place (`op a, number` and `op a, b`).

Both take a rounding mode, a sign mode, or both:

| Mode | Meaning |
| ---- | ------- |
| `round` | to the nearest whole number |
| `floor` | down |
| `ceil` | up |
| `+` | make positive (absolute value) |
| `-` | make negative |

They combine as `floor+`, `round-` and so on. The short forms `r`, `f` and `c` also work, but full names are preferred for readability.

`fldiv` is `div` with rounding down built in. These pairs do the same thing:

```nasm
floor_division:
    fldiv c1, 4
    div c2, 4                   | finmode:floor
    fldiv c3, c1, c2
    div c4, c1, c2              | resmode:floor
```

Sign modes give you an absolute value in a single instruction:

```nasm
absolute_value:
    mov c1, c1                  | resmode:+     ; c1 = |c1|
    sub c3, c1, c2              | resmode:+     ; c3 = |c1 - c2|
```

> [!WARNING]
> An unknown rounding mode is not an error; it is treated as "no rounding". If rounding seems to have no effect, check the spelling.

## 2.6. More arithmetic flags
Each arithmetic instruction compiles to one Item Edit trigger, and flags give you access to more of its options.

**`itemmod`** sets the trigger's modifier. The item result is multiplied by it before it is combined with the target:

```nasm
itemmod_examples:
    add c1, c2                  | itemmod:0.5               ; c1 = (c1 + c2) * 0.5
    sub c1, c2                  | itemmod:3                 ; c1 = (c1 - c2) * 3
    add c1, c2                  | itemmod:4 divmod:true     ; c1 = (c1 + c2) / 4
```

`divmod:true` divides by the modifier instead of multiplying.

**`iter`** changes how the result is combined with the target. The three-item forms normally assign (`=`), so `iter:+` turns them into "add the result to the target":

```nasm
accumulate:
    mul c1, c2, c3              | iter:+        ; c1 += c2 * c3
```

[`bigint.tasm`](https://github.com/ArrowSlashArrow/tasm-lang/blob/master/example_programs/bigint.tasm) uses this to sum products in long multiplication.

**`op`** changes the operator between the two input items of a three-item form: `add c1, c2, c3 | op:*` computes `c2 * c3`. It is rarely clearer than using the matching instruction.

Flags go after a `|`, separated by spaces. A flag and its value are separated by a `:` without spaces. The style guide lines up the `|` of all lines in a routine in one column, as in the examples above.

## 2.7. Example: clock
This program splits a number of seconds into hours, minutes and seconds. It uses the **remainder** pattern: `x mod y` is `x - floor(x / y) * y`.

```nasm clock.tasm
; Splits a number of seconds into hours, minutes and seconds.
_init:
    alias total,            c1
    alias hours,            c2
    alias minutes,          c3
    alias seconds,          c4
    alias rest,             c5      ; seconds left after taking out the hours
    alias INPUT,            9876    ; 2 h 44 min 36 s
    alias SECONDS_PER_HOUR, 3600
    alias SECONDS_PER_MIN,  60

    display total
    display hours
    display minutes
    display seconds

_start:
    mov total, INPUT

    ; hours = floor(total / 3600)
    fldiv hours, total, SECONDS_PER_HOUR

    ; rest = total mod 3600
    mul rest, hours, SECONDS_PER_HOUR
    sub rest, total, rest

    ; minutes = floor(rest / 60), seconds = rest mod 60
    mov minutes, rest
    fldiv minutes, SECONDS_PER_MIN
    mul seconds, minutes, SECONDS_PER_MIN
    sub seconds, rest, seconds
```

After it runs, the displays show `9876`, `2`, `44` and `36`. In [chapter 6](06-functions-and-remaps.md), the remainder calculation becomes a reusable routine.

## 2.8. Exercises
1. Convert a temperature in Fahrenheit (in `t1`) to Celsius (in `t2`): `C = (F - 32) / 1.8`.
2. Split a three-digit number into its digits, and store their sum. For 472, the digits are 4, 7 and 2, and the sum is 13.
3. [Project Euler 6](https://projecteuler.net/problem=6) asks for the difference between the square of the sum and the sum of the squares of the first *n* numbers. Compute it for *n* = 50 without a loop, using `1 + ... + n = n(n+1)/2` and `1² + ... + n² = n(n+1)(2n+1)/6`. The answer is 1,582,700.

Solution 1
```nasm to_celsius.tasm
_init:
    alias fahrenheit,   t1
    alias celsius,      t2
    alias START_TEMP,   98.6

    display fahrenheit
    display celsius

_start:
    mov fahrenheit, START_TEMP
    mov celsius, fahrenheit
    sub celsius, 32
    div celsius, 1.8            ; 37
```


Solution 2
```nasm digits.tasm
_init:
    alias number,       c1
    alias hundreds,     c2
    alias tens,         c3
    alias ones,         c4
    alias digit_sum,    c5
    alias scratch,      c6
    alias INPUT,        472

    display number
    display digit_sum

_start:
    mov number, INPUT

    ; hundreds = floor(n / 100)
    mov hundreds, number
    fldiv hundreds, 100

    ; tens = floor(n / 10) - hundreds * 10
    mov tens, number
    fldiv tens, 10
    mul scratch, hundreds, 10
    sub tens, scratch

    ; ones = n - floor(n / 10) * 10
    mov ones, number
    fldiv ones, 10
    mul ones, 10
    sub ones, number, ones

    add digit_sum, hundreds, tens
    add digit_sum, ones
```


Solution 3
```nasm euler_6.tasm
_init:
    alias n,            c1
    alias sum,          c2      ; 1 + ... + n
    alias square_sums,  c3      ; 1² + ... + n²
    alias answer,       c4
    alias scratch,      c5
    alias N,            50

    display answer

_start:
    mov n, N

    ; sum = n(n + 1) / 2
    mov scratch, n
    add scratch, 1
    mul sum, n, scratch
    div sum, 2

    ; square_sums = n(n + 1)(2n + 1) / 6
    mul square_sums, n, scratch     ; n(n + 1)
    mul scratch, n, 2
    add scratch, 1                  ; 2n + 1
    mul square_sums, scratch
    div square_sums, 6

    ; answer = sum² - square_sums
    mul answer, sum, sum
    sub answer, square_sums         ; 1582700
```
For *n* = 100, `sum²` is 25,502,500, which is above 2<sup>24</sup> and may not be exact.


So far every program has run from top to bottom once. In the next chapter, you will make routines run each other, wait, and run side by side.
