# Unresolved issues

An audit of the TASM codebase. It covers the compiler (`rtasm/src`, excluding `decomp.rs`), the standard library, example programs, docs, install scripts, CI and the VS Code extension. The decompiler and `tasm-lsp` were excluded as experimental.

The compiler audit reflects the working tree as of 2026-10-10, including the uncommitted removal of `ADDM`/`SUBM`/`ADDD`/`SUBD` and the `IMPORTSTD` path fix. Items marked **(verified)** were reproduced by compiling a small probe program. The rest come from reading the code.

Severity: 🔴 wrong output or crash · 🟠 confusing or silently wrong behaviour · 🟡 design/consistency · ⚪ cosmetic/cleanup.

## Contents
1. [Compiler: group allocation](#1-compiler-group-allocation)
2. [Compiler: lexer and value parsing](#2-compiler-lexer-and-value-parsing)
3. [Compiler: flags](#3-compiler-flags)
4. [Compiler: instruction handlers](#4-compiler-instruction-handlers)
5. [Compiler: linker and modules](#5-compiler-linker-and-modules)
6. [Compiler: errors and diagnostics](#6-compiler-errors-and-diagnostics)
7. [Compiler: CLI and export](#7-compiler-cli-and-export)
8. [Language design and syntax](#8-language-design-and-syntax)
9. [Standard library and example programs](#9-standard-library-and-example-programs)
10. [Tests](#10-tests)
11. [Documentation](#11-documentation)
12. [Install scripts, CI and tooling](#12-install-scripts-ci-and-tooling)

---

## 1. Compiler: group allocation

### 1.1 🔴 Auxiliary groups collide with routine groups and with each other
**Files:** `rtasm/src/core/mod.rs:237`, `rtasm/src/instr/fns.rs:364-371, 467-487, 541, 579-580`, `rtasm/src/main.rs` (`parse_main` return value)

**Problem:** Base compares and `SRAND`/`FRAND` put their intermediate spawn triggers in "auxiliary" groups that start at `self.curr_group`. That value is the group of the **last routine** (the counter is not advanced past it), and handlers don't agree on how to use it:
- Spawn compares and `SRAND` use `curr_group` (and `+1` for `FRAND`), then advance by 1 or 2.
- Fork compares use `curr_group + 1` and `curr_group + 2`, then advance by 2. The *next* instruction's first aux group is therefore the fork's `+2` group.

So:
- The first spawn compare in a program shares a group with the last routine. Spawning that routine also fires the compare's spawn trigger.
- A fork followed by any other base compare shares a group. For example, after `FE a, b, ...` then `SE c, ...`, FE's "true" spawn trigger and SE's spawn trigger end up in the same group, so either branch fires both.

**Potential fixes:**
- Start auxiliary allocation at `last_routine_group + 1`.
- Make every handler use the half-open range `[curr_group, curr_group + n)` and advance by `n`. Better still, have handlers *request* groups from an allocator instead of computing offsets themselves.
- Add a test that compiles a program with consecutive compares and asserts that every aux group is unique and distinct from the routine groups.

**Possibly linked bugs:** Spurious spawns, double execution, and "random" misbehaviour in programs with many branches. It may explain some race-like behaviour noted in comments (e.g. `example_programs/list_search.tasm`, which needs `kill` twice). The routine-group collision was observed in an earlier session: the SE spawn trigger was given the same group as routine `a`.

### 1.2 🔴 An empty `_init` routine gives the next routine group 0 (verified)
**File:** `rtasm/src/lexer.rs:597-615`

**Problem:** `index_routines` decrements `curr_group` once because the routine is `_init`, and again because it is empty. With `_init:` followed by a blank line and `_start:`, `_start` gets group **0** and the compiler prints `Using groups 1 - 0`. `post_link_processing` then dedups routines by group (`linker.rs:661-662`), so a group-0 routine can silently replace another one.

**Potential fixes:** Restructure the "commit previous routine" logic so that each routine is decided once (init → no group; empty → no group; else → next group). Don't do it with paired decrements. Assert `group >= 1` for non-init routines.

**Possibly linked bugs:** 2.2, 5.6.

### 1.3 🔴 Spawning an empty routine spawns the *next* routine (verified)
**File:** `rtasm/src/lexer.rs:604-615`

**Problem:** An empty routine's name is inserted into `routine_group_map` *before* its group is given back to the counter. The next routine then reuses that group. So for `empty:` (no body) followed by `b: ...`, `SPAWN empty` compiles to `SPAWN g<b>`. The docs say empty routines are skipped, and the book recommends empty routines as "do nothing" branch targets, so this is actively dangerous.

**Potential fixes:**
- Either don't register empty routines and emit an error ("routine `x` has no instructions") when they are referenced,
- or give them a group and place nothing (a cheap, valid "do nothing" target).
- `stdlib/mem_8bit.tasm::mem_end` relies on a placeholder routine. Check that it still works after the fix (it currently contains `nop`, which avoids the bug).

**Possibly linked bugs:** Any `IFE x, nothing, ...` pattern where `nothing:` is empty.

### 1.4 🟠 Group limit is checked before, not after, auxiliary groups
**File:** `rtasm/src/core/mod.rs:93-102`

**Problem:** `curr_group > GROUP_LIMIT` is only checked at the start of each routine. Auxiliary groups allocated by the last routine's instructions can exceed 9,999 without E0001. The check also happens during placement, so it can't account for all groups up front.

**Potential fixes:** Check after allocating groups for every instruction, or compute the total before placement.

### 1.5 🟠 Imported modules reserve groups for routines that are never linked
**File:** `rtasm/src/linker.rs:154-156`

**Problem:** `start_using_this_group` is advanced by *every* routine in an imported module, but only the referenced routines (and their dependencies) are copied in. This leaves holes in the group range. For example, `example_programs/stdlib_modulo.tasm` reports `Using groups 1 - 12` but only uses 7 routine groups.

**Potential fixes:** Assign groups lazily, after reachability is known (a separate "assign groups" pass after linking).

**Possibly linked bugs:** 5.2 (unreferenced routines are also linked).

### 1.6 🟠 `Using groups X - Y` excludes auxiliary groups
**File:** `rtasm/src/main.rs:264-269`

**Problem:** The printed range only covers routine groups. Users choosing `--group-offset` or placing their own objects can't tell which groups the program really uses.

**Potential fixes:** Print the true final range, e.g. `Using groups 1 - 5 (routines), 6 - 11 (compare helpers)`.

### 1.7 🟠 `--group-offset` accepts negative values (verified)
**File:** `rtasm/src/main.rs` (`group_offset: i16`)

**Problem:** `--group-offset=-5` compiles and prints `Using groups -4 - -4`. Negative and zero group IDs are invalid in GD.

**Potential fixes:** Use `clap::value_parser!(i16).range(0..GROUP_LIMIT)`.

---

## 2. Compiler: lexer and value parsing

<!-- ### 2.1 🔴 `ALIAS` with one argument panics (verified)
**File:** `rtasm/src/lexer.rs:201-232`

**Problem:** E0012 is pushed when the argument count is wrong, but execution continues and indexes `trimmed[1]`. The input `ALIAS foo` crashes with `index out of bounds`. The same pattern exists for `IMPORT` (`lexer.rs:135-148`), which keeps using `trimmed[0]` after the error.

**Potential fixes:** `continue` after pushing the error. -->

### 2.2 🔴 Panic on a non-ASCII character after `C`/`T`/`G`/`b` (verified)
**File:** `rtasm/src/core/structs.rs:167-177`

**Problem:** `&remaining[0..1]` and `&remaining[1..]` slice by byte. Any token whose first character is `c`, `t`, `g` or `b` and whose second character is multi-byte panics. For example, the IOBlock label `té` gives `byte index 1 is not a char boundary`. These slices are computed eagerly for every token.

**Potential fixes:** Use `remaining.chars().next()` and `char_indices`, or only compute the hex path when `remaining.starts_with('x')`.

### 2.3 🔴 Instructions with no arguments can't take flags (verified)
**File:** `rtasm/src/lexer.rs:378, 471-480`

**Problem:** For `RPERSALL | ordered:false`, the left side is `"RPERSALL "` with a trailing space. `args_string.trim().find(" ")` finds nothing, and the untrimmed string is uppercased into `"RPERSALL "`. That fails with `E0025 Unrecognized instruction RPERSALL ` (note the trailing space). This affects `NOP`, `BREAKPOINT`, `RPERSALL` and `UNPERSALL`.

**Potential fixes:** Trim `args_string` once at the start and use the trimmed value everywhere (including the slicing at line 392, which currently indexes the untrimmed string with a position from the trimmed one).

**Possibly linked bugs:** The slicing mismatch in `args_string[..pos]` vs `args_string.trim().find(" ")` would also break if `args_string` ever had leading whitespace.

### 2.4 🟠 Unterminated dict flags are silently dropped (verified)
**File:** `rtasm/src/lexer.rs:717-783`

**Problem:** If a dict never closes (`remap: {1=2`), `in_dict` stays true when the loop ends, and the flag is discarded with no error. The `SPAWN` compiles without the remap.

**Potential fixes:** After the loop, emit E0030/E0032 if `in_dict` is still true.

### 2.5 🟠 Malformed dicts can panic or give unhelpful errors
**File:** `rtasm/src/core/flags.rs:200-201`

**Problem:** The dict body is taken as `value[1..value.len() - 1]` without checking that it starts with `{` and ends with `}`. A value of `}` (length 1) slices `[1..0]` and panics. Missing braces give a generic E0032.

**Potential fixes:** Use `strip_prefix('{')` / `strip_suffix('}')` and return a specific error.

### 2.6 🟠 Routine names are not validated
**File:** `rtasm/src/lexer.rs:618-654`

**Problem:** Any unindented token ending in `:` becomes a routine: `c5:`, `g2:`, `inf:`, `nan:`, `a::b:`, `123:`. When referenced, these names parse as a counter, group, number or external symbol instead of the routine. A label with an instruction on the same line (`loop: ADD C1, 1`) gives a bare `E0029 Bad token.` with no line number (see 6.3).

**Potential fixes:**
- Validate routine names at declaration (`[A-Za-z_][A-Za-z0-9_]*`, and not parseable as another value), with a clear error.
- Give `label: instr` a specific error, or support it.

### 2.7 🟠 Aliases silently shadow routines and built-ins
**File:** `rtasm/src/lexer.rs:184-251, 444-450`

**Problem:** Alias substitution happens before routine lookup, so `ALIAS a, C5` plus a routine `a:` makes `SPAWN a` fail with a confusing E0027 `[Item]`. E0014 ("alias identifier is a built-in") is documented but never emitted, so `ALIAS POINTS, C5` is accepted and silently rewrites every later use of `POINTS`.

**Potential fixes:** Reject alias names that collide with routine names or built-ins (emit E0014), and validate alias names with the same rules as routine names.

### 2.8 🟠 Indented lines before the first routine are silently ignored (verified)
**File:** `rtasm/src/lexer.rs:668-674`

**Problem:** An indented instruction at the top of a file (before any label) is dropped with no warning.

**Potential fixes:** Emit an error ("instruction outside of a routine").

### 2.9 🟠 Item and group IDs above 9,999 are accepted; out-of-range `i16` IDs become strings (verified)
**File:** `rtasm/src/core/structs.rs:209-226`

**Problem:**
- Only `id <= 0` is rejected, yet the message says the range is `[1, GROUP_LIMIT]`. `C20000` compiles.
- `C40000` overflows `i16`, falls back to `String`, and produces a confusing E0027 (`[String, Number]`).
- `b0` and `b-3` are accepted as collision blocks with no range check (verified).

**Potential fixes:** Parse the ID as a wider integer, then range-check to `[1, 9999]` with E0017 for every prefix, including `b`.

**Possibly linked bugs:** Docs say IDs above 9,999 are unsupported, but the compiler is silent about them.

### 2.10 🟠 Non-finite numbers are accepted as arguments (verified)
**File:** `rtasm/src/core/structs.rs:240-242`

**Problem:** `nan`, `inf` and `infinity` parse as numbers (`MOV C1, nan` compiles). Float *flags* reject non-finite values (`flags.rs:152`), so arguments and flags are inconsistent.

**Potential fixes:** Reject non-finite numbers in `to_value`, or document them.

### 2.11 🟡 Inconsistent hex syntax
**File:** `rtasm/src/core/structs.rs:173-177, 243-253`

**Problem:** Numbers use `0x1F`, but IDs use `Cx1F` (and `C0x1F` is a string). `0X1F` (uppercase X) isn't supported, negative hex isn't supported, and `bx5` isn't supported for collision blocks.

**Potential fixes:** Accept `C0x1F` (and drop or deprecate `Cx1F`), and allow `0X`.

### 2.12 🟡 `\` escape doesn't stop routine resolution
**File:** `rtasm/src/lexer.rs:817-843`

**Problem:** `\foo` becomes `String("foo")`, and `validate_tasm_value` then turns any string that matches a routine name into a `RoutineRef`. An escaped IOBlock label that matches a routine name therefore fails to compile.

**Potential fixes:** Add a separate `TasmValue::Literal` (or a flag) for escaped strings that skips routine resolution.

### 2.13 🟡 Nested `::` paths silently become strings (verified)
**File:** `rtasm/src/core/structs.rs:182-189, 374-390`

**Problem:** `a::b::c` fails `split_at_str_once` and falls through to `String`, giving E0027 `[String]` instead of "nested module paths are not supported".

### 2.14 🟡 `IMPORT` failures are partly silent and lose `_init`
**File:** `rtasm/src/lexer.rs:121-181`

**Problem:**
- If `TasmValue::to_value` fails (e.g. `IMPORT c0`), the import is silently skipped.
- On a non-path argument the function `return`s early **without restoring** `routine_data[0].lines`, which were taken with `mem::take`. The rest of `_init` disappears.
- Flags on `IMPORT`/`ALIAS` lines are silently ignored.

**Potential fixes:** Use `continue` instead of `return`, report every failure, and reject flags on directives.

### 2.15 🟡 Byte-based keyword stripping
**File:** `rtasm/src/lexer.rs:124-131, 193-197`

**Problem:** `IMPORT`/`IMPORTSTD`/`ALIAS` detection uses `to_uppercase().starts_with("IMPORT ")` and then slices fixed byte offsets (7/10/6). It works for ASCII, but it's fragile and duplicates the instruction tokenizer. `~ALIAS` (concurrent alias) is silently accepted.

**Potential fixes:** Tokenize directives with the same code path as instructions.

### 2.16 🟡 `stdlib_dir()` panics without `APPDATA`/`HOME`, and has no override
**File:** `rtasm/src/lexer.rs:883-897`

**Problem:**
- It uses `expect(...)`, so it panics in stripped-down environments.
- There is no `TASM_HOME`/`TASM_STDLIB` environment override.
- There is no fallback to a `stdlib/` folder next to the executable, which is how release archives are laid out for users who don't run the installer.
- Linux uses `~/.local/bin/tasm/stdlib` (see 12.2).

**Potential fixes:** Return an error instead of panicking. Look in `$TASM_STDLIB`, then next to `current_exe()`, then the install directory.

### 2.17 ⚪ Outdated module docs in `lexer.rs`
**File:** `rtasm/src/lexer.rs:1-38`

**Problem:** The doc comment mentions `MALLOC` and memory cells (removed), says instructions are "separated by 1 unit" (only in release mode), and says `_start` is "assigned group 1" (it's assigned in declaration order). It also says to indent "by at least 2 or 4 spaces" (any indentation works).

### 2.18 ⚪ Dead or misleading code in `structs.rs`
**File:** `rtasm/src/core/structs.rs`

**Problem:**
- `TasmValue::UnresolvedAlias` is a "temporary state" that is never constructed.
- `Routine::empty` and the "legacy: N" comments on `RoutineData` are leftovers.
- `HandlerData` comments mention `malloc`.
- `InstrType::Debug` says "only used by the emulator" (the emulator is legacy).
- `Tasm.lines` keeps the whole file.
- The `unsafe { unreachable_unchecked() }` calls (lines 201, 235) are unnecessary and become undefined behaviour if the match arms are ever edited. Use `unreachable!()`.

---

## 3. Compiler: flags

### 3.1 🟠 Flags aren't validated per instruction
**Files:** `rtasm/src/core/flags.rs:288-316`, `rtasm/src/instr/mod.rs`

**Problem:** `get_flag_type` is one global registry, so a flag that an instruction doesn't read is silently ignored (e.g. `MOV C1, 2 | ordered:false`, or `delay` on a compare). Typos in valid-looking positions go unnoticed.

**Potential fixes:** Add the accepted flags to each `INSTR_SPEC` entry and emit an error (or warning) for unused flags. This also documents flags in code.

### 3.2 🔴 `COLL` reads unregistered flags
**File:** `rtasm/src/instr/fns.rs:964-969`, `rtasm/src/core/flags.rs:288-316`

**Problem:** `coll` reads `nospawn` and `onexit`, but they are missing from `get_flag_type`, so using them gives E0031. `docs/index.md:204, 562-563` advertises them.

**Potential fixes:** Add `"nospawn" | "onexit" => Bool` to `get_flag_type`.

### 3.3 🟠 Duplicate flags are silently accepted (verified)
**File:** `rtasm/src/core/mod.rs:223-227`

**Problem:** `ADD C1, 2 | itemmod:3 itemmod:4` compiles. The flags are collected into a `HashMap`, so the last one wins silently.

**Potential fixes:** Emit an error on duplicate flag names.

### 3.4 🟠 Bad flag spacing produces misleading errors
**File:** `rtasm/src/lexer.rs:714-783`

**Problem:** `itemmod: 0.5` reports `E0031 Unrecognized flag 0.5`, but the real problem is the space after the colon. Meanwhile `remap: {...}` *requires* a space when it uses `::`. The rules depend on the flag type.

**Potential fixes:**
- Make flag parsing a real tokenizer: allow optional whitespace after `:`, and split on the *first* `:` only, so `remap:{a::b=1}` works.
- Failing that, detect "segment ends with `:`" and report "space after ':'".

### 3.5 🟡 Round/sign modes fail silently
**File:** `rtasm/src/core/flags.rs:50-93`

**Problem:** Unrecognized round modes (`flor`, `+round`, `roundabs`) become "no rounding" with no error.

**Potential fixes:** Return `None` (E0032) for unknown modes.

### 3.6 🟡 Aliases work in dict flags but not scalar flags
**File:** `rtasm/src/core/flags.rs:149-166` vs `167-224`

**Problem:** `remap: {A = B}` resolves aliases, but `itemmod:HALF` fails with E0032. Users have to remember which flags accept aliases.

**Potential fixes:** Resolve aliases for every flag value type.

### 3.7 ⚪ `From<FlagValue>` impls `unwrap`
**File:** `rtasm/src/core/flags.rs:95-119`

**Problem:** These panic if the stored type doesn't match. They are only safe because `get_flag_type` maps names to types globally; a flag reused with a different type would crash. Prefer `TryFrom`.

### 3.8 🟡 Remap dict keys and values are limited to `i16`
**File:** `rtasm/src/core/flags.rs:121-139`

**Problem:** `0x` hex values over `i16::MAX` and negative hex fail. That's acceptable for IDs, but the error message doesn't say why.

---

## 4. Compiler: instruction handlers

<!-- ### 4.1 🔴 3-argument `DIV`/`FLDIV` with a number multiplies 
**File:** `rtasm/src/instr/fns.rs:226-261`

**Problem:** `arithmetic_2items_num` always sets `multiply_mod = !divmod` (default `true`). `DIV C1, C2, 4` compiles to `C1 = C2 * 4`. The workaround is `| divmod:true`.

**Potential fixes:** Default `divmod` to `op == Op::Div`.

**Possibly linked bugs:** `stdlib/compare.tasm` (`div c3, c3, 2 | resmode:+ iter:/`) may depend on the current multiply behaviour (`d / (|d|*2)`). Check and adjust it when fixing.-->

### 4.2 🟠 `ADDM`/`SUBM`/`ADDD`/`SUBD` removed but still referenced (uncommitted change)
**Files:** `rtasm/src/instr/mod.rs`, `rtasm/src/instr/fns.rs` (working tree), `tests/all_instructions.tasm:19-22`, `docs/index.md:100-115`, `docs/updated_docs_v1.md` §3.1.2.1.2, `docs/book/09-debugging.md` §9.5, `vscode-ext/syntaxes/tasm.tmLanguage.json`

**Problem:** The instructions are gone from `INSTR_SPEC`, so `cargo test` fails (`compile_success_all_instructions`). The docs, book and syntax highlighting still describe them, including the old "modifier swapped" bug.

**Potential fixes:** Remove them from the test, docs, book and grammar, and mention the replacement (`ADD ... | itemmod:x [divmod:true]`). Or restore them with the modifier fixed (they previously compiled with the multiply/divide modifier swapped).

### 4.3 🟡 `WAITS` is scaled by the build mode but `WAIT`/`NOP` aren't
**File:** `rtasm/src/instr/fns.rs:99-108`

**Problem:** `WAIT n` means "n spaces" (n ticks in release, 30n in debug), while `WAITS s` means real time in both modes. The same program can have different relative timing between debug and release. `WAITS` also truncates (`as i32`), so `WAITS 0.003` is 0 ticks with no warning.

**Potential fixes:** Pick one model (spaces or real time) and document it, or add a separate debug-mode time-scale flag. Warn when `WAITS` rounds to 0.

### 4.4 🟡 IOBlock positions aren't validated
**File:** `rtasm/src/instr/fns.rs:763-790`

**Problem:** Negative positions, position 0 (which overlaps the automatic `start` block) and duplicate positions are accepted silently.

**Potential fixes:** Error or warn on position ≤ 0 and duplicates.

### 4.5 ⚪ Object overlaps in debug mode
**Files:** `rtasm/src/core/mod.rs:204-214`, `rtasm/src/instr/fns.rs:743-747`

**Problem:** In debug mode, `_init` triggers are placed at `x = -15 - n*30, y = 75`. `DISPLAY` labels sit at `x = -75, y = 75 + 30k`, so the third `_init` trigger overlaps the first display label.

**Potential fixes:** Give `_init` triggers their own row.

### 4.6 🟡 `RAWTRG` in `_init` adds group 0
**File:** `rtasm/src/instr/fns.rs:878-901`

**Problem:** `args.cfg.groups.get(0).unwrap_or(&Group::Regular(0))` adds group 0 to the objects when used in `_init`.

**Potential fixes:** Skip `add_group` when there is no routine group.

### 4.7 🟡 `RAW`/`RAWTRG` can't contain `;` or `|`, and tabs are rewritten
**Files:** `rtasm/src/lexer.rs:858-863`, `rtasm/src/instr/fns.rs:866-901`

**Problem:** Comment stripping happens before tokenizing, so object strings keep only one object. The handlers' `.split(';')` is dead code. Tabs are also converted to spaces inside raw strings. Malformed object strings are passed to `GDObject::parse_str` unchecked.

**Potential fixes:** Strip comments only outside raw arguments (e.g. `RAW` takes the rest of the raw line), or support a quoted or block form. Validate the object strings.

### 4.8 🟡 `EVENT` silently truncates and defaults its flags
**File:** `rtasm/src/instr/fns.rs:993-1024`

**Problem:**
- `extra1` is cast with `as i16` (no range check).
- An invalid `extra2` becomes the default via `unwrap_or_default()` with no error.
- Only one event per trigger is supported, although GD event triggers accept several.
- E0019 is reused here while `structs.rs:241` says "errcode 18 and 19 are defunct".

### 4.9 ⚪ Misleading internal names
**File:** `rtasm/src/instr/mod.rs:477-497`, `rtasm/src/instr/fns.rs`

**Problem:**
- `LowerCompOp::le` means **less than** and `leq` means less-or-equal (likewise `ge`/`geq`). `le` conventionally means `<=`.
- `KILL` maps to `stop`, and `TOGGLEON`/`TOGGLEOFF` map to `ton`/`toff`.
- `GROUP_SPAWN_DELAY` (0.0044 s) is documented as "length of 1 game tick", but a tick is 1/240 ≈ 0.004167 s.
- `placeholder_panic_fn` doesn't panic (it returns an error with code `-1`, which prints as `[E-001]`).

### 4.10 🟡 Inconsistent argument order across instructions
**File:** `rtasm/src/instr/mod.rs:353-357`

**Problem:** Most instructions take the target group first (`SE g, a, b`, `COLL g, h1, h2`), but `TSPAWN` takes it last (`TSPAWN T1, start, end, g`). `IOBLOCK` takes `group, position, label`, while `EVENT` takes `group, id`. Arithmetic puts the result first.

**Potential fixes:** Document the convention ("targets first"), and consider reordering `TSPAWN` in a minor release with a deprecation path.

---

## 5. Compiler: linker and modules

### 5.1 🔴 Absolute file paths are written into the level
**Files:** `rtasm/src/linker.rs:329-337`, `rtasm/src/core/mod.rs:109-118`

**Problem:** Imported routines are renamed to `<canonical path>::<ident>`. That name is used for the routine label text in the level, e.g. `2: \\?\C:\Users\<name>\...\math.tasm::modulo`. This leaks the author's username and directory layout into shared levels, and makes labels very long.

**Potential fixes:** Keep the mangled name for internal uniqueness, but display `module::ident` (stem plus identifier) in labels and messages.

### 5.2 🟠 Linking is not lazy for references inside unused routines
**File:** `rtasm/src/linker.rs:165-171`

**Problem:** `parse_module` scans *every* routine of an imported module for external symbols and links all of them. Importing `math` and using only `math::modulo` also links `compare::greater`/`less`, because the unused `math::sign` references them. The docs claim imports are lazy.

**Potential fixes:** Start from the routines that are actually referenced and walk dependencies (the logic already exists in `index_routine_deps`).

**Possibly linked bugs:** 1.5 (group holes), 1.4 (group limit).

### 5.3 🟠 Module identity is the file stem, and it is global
**Files:** `rtasm/src/linker.rs:251, 398-434`

**Problem:**
- `import a/utils` and `import b/utils` can't both be used: `resolve_dependency_path` picks the first stem match.
- `dependency_map` is keyed by stem across *all* modules. If two different modules each import a different `utils.tasm`, remap resolution in `post_link_processing` can use the wrong one.
- There is no `import ... as name`.

**Potential fixes:** Key dependencies by `(importing module, stem)` or by canonical path, add `IMPORT path AS name`, and error on ambiguous stems.

### 5.4 🟠 Linker errors stop at the first error and use ad-hoc formats
**Files:** `rtasm/src/linker.rs` (many `anyhow!` returns), `rtasm/src/main.rs:161-186`

**Problem:**
- Linking stops at the first error (`?`).
- Errors are formatted strings with hand-written `[E00NN]`, no line numbers or routine names, in three different styles.
- The final message always says `Unable to parse main module: lexing failed`, even when linking failed.

**Potential fixes:** Return `Vec<TasmError>` from the linker, and include the file, line and routine of the referencing instruction.

### 5.5 🟠 Errors in imported modules can be reported against the wrong file
**File:** `rtasm/src/linker.rs:718`

**Problem:** `find_handler_for_instr(&instr, &module.fname, ...)` uses the *main* module's file name for instructions that came from a library, together with the library's line number.

**Potential fixes:** Store the source file on each `Routine` (or `Instruction`).

### 5.6 🟠 Duplicate removal relies on unique group numbers
**File:** `rtasm/src/linker.rs:660-662`

**Problem:** Routines are deduplicated with `dedup_by(|a, b| a.group == b.group)`. This silently drops a routine if two distinct routines ever share a group (see 1.2).

**Potential fixes:** Deduplicate by mangled identity, and assert that groups are unique.

### 5.7 🟡 Imported aliases aren't resolved like local ones
**Files:** `rtasm/src/linker.rs:295-316, 686-707`

**Problem:** External alias values go through `TasmValue::to_value` only. An alias in a library whose value is a routine name becomes a `String`, not that library's routine. Local aliases do resolve to routines.

### 5.8 🟡 `PathBuf::add_extension` may need a newer Rust than declared
**File:** `rtasm/src/linker.rs:413`, `rtasm/Cargo.toml` (`rust-version = "1.88"`)

**Problem:** `PathBuf::add_extension` was stabilized after 1.88 (believed to be 1.91), so the MSRV in `Cargo.toml` and README may be wrong. Not verified here.

**Potential fixes:** Run `cargo +1.88 check`, then either bump `rust-version` or use `set_extension` / `with_extension`.

### 5.9 ⚪ Windows verbatim paths in output
**Files:** `rtasm/src/main.rs:145`, `rtasm/src/linker.rs:414`

**Problem:** `canonicalize()` produces `\\?\C:\...` paths, which appear in every error message. Error headers also print them Debug-escaped (`"\\\\?\\C:\\..."`).

**Potential fixes:** Use the `dunce` crate, or strip the prefix for display. Print paths with `Display`.

---

## 6. Compiler: errors and diagnostics

### 6.1 🔴 `tasmc` exits with status 0 on compile errors (verified)
**File:** `rtasm/src/main.rs:192-311`

**Problem:** `main` returns `()` on every path, so scripts and CI can't detect failures.

**Potential fixes:** Return `std::process::ExitCode` (1 on any error).

### 6.2 🟠 Errors on the first line of a file lose their location (verified)
**File:** `rtasm/src/core/error.rs:223-245`

**Problem:** `line == 0` means "no line", but line numbers are 0-based, so an error on line 1 prints without file position or routine.

**Potential fixes:** Make `line: Option<usize>`.

### 6.3 🟠 Unhelpful messages
**Files:** `rtasm/src/lexer.rs:341, 466, 664`, `rtasm/src/core/error.rs`

**Problem:**
- `Bad token.` (E0029) doesn't show the token or explain what was expected.
- E0021 says "Bad flag arguments" instead of "more than one `|`".
- E0024 doesn't name the bad argument, and is pushed *in addition to* the specific error.
- E0027/E0032 print Rust `Debug` type names (`[Item, Number]`, `Float`).

**Potential fixes:** Include the offending text and a hint, and use TASM type names (`<item>`, `<number>`).

### 6.4 🟡 Error codes reused for different situations, or missing docs
**Files:** `rtasm/src/lexer.rs:144, 159, 210`, `rtasm/src/linker.rs:211, 418, 701`, `rtasm/src/instr/fns.rs:916, 958`, `rtasm/src/core/error.rs`

**Problem:**
- E0043 is used for "flag references a missing routine", "unable to resolve module path" and "unable to find routine".
- E0034 is used for "import outside `_init`" and "argument is not a module path" (whose message wrongly says "must only have one argument").
- E0027 and E0042 are the same error.
- E0016 and E0022 are both "trailing comma".
- E0036 and E0037 have the same title.
- E0048 (bad hitbox pair) is emitted but missing from `ERROR_DOCS`, so `tasmc -e 48` prints nothing useful.
- E0014 is documented but never emitted.
- Placeholder codes E0005, E0007–E0010 and E0018 are not marked deprecated.

**Potential fixes:** One code per situation. Generate `ERROR_DOCS` from an enum so that every emitted code is documented.

### 6.5 🟡 `TasmErrorType` misuse and typos
**File:** `rtasm/src/core/error.rs`

**Problem:**
- E0021 (a flag error) uses `InvalidInstruction`, and E0015 (bad alias) uses `InvalidInstruction`.
- `MultipleRoutineDefintions` is misspelled.
- `InvalidNumber` is commented out.
- Doc typos: E0035 "This errors is", E0045 "thorugh", E0031 "E0025 for flags".
- E0038's doc says "file can't be found", but it is emitted when a `mod::sym` names a module that was never imported.

### 6.6 🟡 Verbose and normal logging are tangled
**Files:** `rtasm/src/lexer.rs:874-878`, `rtasm/src/linker.rs:144, 163, 364, 455`

**Problem:** `log!(silent, ...)` is called with `silent = verbose && !no_log`, so the variable name says the opposite of what it means. Some debug prints (`called cache module with symbol ...`) are user-facing in verbose mode. With `--no-log`, a failed compile prints nothing and still exits 0.

**Potential fixes:** Rename the variable to `verbose`, use a logging crate (`log`/`env_logger`), and always print errors to stderr.

### 6.7 🟡 Errors go to stdout
**Files:** `rtasm/src/core/mod.rs:305-310`, `rtasm/src/main.rs`

**Problem:** All diagnostics use `println!`. Use `eprintln!` so that `tasmc -d file > out.tasm` and similar redirections stay clean.

---

## 7. Compiler: CLI and export

### 7.1 🟠 Savefile not found is reported as success
**File:** `rtasm/src/main.rs:131-142`

**Problem:** If the savefile can't be found, the compiler logs a hint and returns `Ok(())`. Combined with 6.1, scripts can't tell that nothing was exported. Writing to the savefile while GD is running is also not detected.

**Potential fixes:** Return an error. Optionally detect a running GD process and refuse or warn.

### 7.2 🟠 `--clipboard` can panic
**File:** `rtasm/src/main.rs:282-291`

**Problem:** `ClipboardContext::new().unwrap()` and `get_decrypted_data().unwrap()` panic on headless or unsupported systems. The docs say the flag is Windows-only, but `cli-clipboard` is cross-platform, and on musl/X11-less builds it may not link or work.

**Potential fixes:** Handle the errors, and clarify the platform support.

### 7.3 🟡 Export options are mutually exclusive but not declared as such
**File:** `rtasm/src/main.rs` (`Args`)

**Problem:** `--clipboard` wins over `--wslive`, which wins over `--gmd`. Passing two of them silently ignores the others.

**Potential fixes:** Use a clap `ArgGroup` with `multiple(false)`, or a single `--output <savefile|gmd|clipboard|wslive>` option.

### 7.4 🟡 `--gmd` output name and location
**File:** `rtasm/src/main.rs:241-244, 300`

**Problem:** The level name defaults to the input path *as typed* (e.g. `../example_programs/x.tasm`). The `.gmd` file is then written as `<that>.gmd` relative to the current directory, and the level is named with the full relative path.

**Potential fixes:** Default the level name to the file stem, and add an `-o/--output` path.

### 7.5 🟡 Platformer mode and GD version are hard-coded
**Files:** `rtasm/src/main.rs:275-280`, `rtasm/src/core/mod.rs:74`

**Problem:** Every level is forced into platformer mode, and the version is fixed to `GD22081`. Neither is configurable.

### 7.6 🟡 WSLive payload is built by string formatting
**File:** `rtasm/src/main.rs:116-123`

**Problem:** The JSON is assembled with `format!` and no escaping. Object strings containing `"` or `\` (possible in text objects or `RAW` input) would produce invalid JSON.

**Potential fixes:** Use `serde_json`.

### 7.7 ⚪ `-e` short flag is ambiguous
**File:** `rtasm/src/main.rs:82-84`

**Problem:** `-e` is `--error-help`, which takes a number. `rtasm/profile.ps1` passes a bare `-e`, which is a CLI error. `error_help: usize` with a default of `0` doubles as "not set".

**Potential fixes:** Use `Option<usize>`, and consider a subcommand (`tasmc explain E0027`) that accepts the `E0027` form as well as `27`.

---

## 8. Language design and syntax

### 8.1 🟡 `;` comments make object strings and some labels impossible
**File:** `rtasm/src/lexer.rs:858-863`

**Problem:** See 4.7. GD object strings are `;`-separated, so `RAW` can only take one object at a time.

### 8.2 🟡 Case sensitivity is inconsistent
**Files:** `rtasm/src/core/structs.rs:190-200`, `rtasm/src/lexer.rs`, `rtasm/src/core/flags.rs`

**Problem:**
- Case-insensitive: instruction names and item prefixes (`c5` = `C5`).
- Case-sensitive: built-ins (`POINTS`), flag names (lowercase only), flag values (`true`), aliases, and routine names.

**Potential fixes:** Pick a rule (e.g. "keywords are case-insensitive, user identifiers are case-sensitive") and apply it to built-ins and flag names too.

### 8.3 🟡 Remaps apply to every ID type, and the language doesn't help
**Files:** `stdlib/*`, `docs`

**Problem:** GD remaps are ID-type agnostic, so `remap: {1 = 3}` changes counter 1, timer 1 and group 1. Libraries pass arguments by remapping low IDs, which can hit routine groups (see 9.1). There is no warning when a remap key equals a group that the spawned routine uses.

**Potential fixes:** Have the compiler warn when a remap key collides with a group used inside the target routine, or its dependencies. Document the convention of reserving high argument IDs.

### 8.4 🟡 No real calling convention or return mechanism
**Files:** `stdlib/core.tasm`, `stdlib/math.tasm`

**Problem:** Calls are "spawn, then `wait n` and hope". The stdlib mixes conventions: `@0tick` unordered spawns, `wait 2` after memory calls, `return_signal` polling in `core::wait_until`, and branch targets passed by remap. Unordered (`ordered:false`) routines also rely on GD activating triggers in placement order, which the docs say isn't guaranteed.

**Potential fixes:** Define a convention for continuations (pass a "continue" group), and possibly add a compiler-supported `CALL routine, continuation` that wires it up.

### 8.5 🟡 `_init` instructions behave differently when imported
**Files:** `rtasm/src/linker.rs`, docs

**Problem:** Only `ALIAS`/`IMPORT` survive when a module is imported. `PERS`, `DISPLAY`, `EVENT` and `IOBLOCK` are silently dropped, with no warning.

**Potential fixes:** Warn when an imported module's `_init` contains instructions that will be ignored.

### 8.6 ⚪ Routine and label syntax limits
**File:** `rtasm/src/lexer.rs:597-654`

**Problem:** A label must be alone on its line. The `_init` routine can't be named in some places (E0033), but it can be referenced by name in remaps without an error.

---

## 9. Standard library and example programs

Every file in `stdlib/`, `example_programs/` and `programs/` was compiled and read. All stdlib files compile with `--no-entry-point`.

### 9.1 🟠 `math::modulo_fork` remaps collide with its own inputs
**File:** `stdlib/math.tasm` (`modulo_fork`)

**Problem:** The routine takes its branch targets as `g1`/`g2` (passed by remap) and its inputs as `C1`/`C2`. Remapping IDs 1 and 2 also remaps `C1` and `C2` (see 8.3). Its doc comment also names the targets `g3`/`g4`.

**Potential fixes:** Use high reserved IDs for the targets (as `vec.tasm` does with 9991–9994), and fix the comment.

### 9.2 🟡 `consts::NEG_INT_LIMIT` is wrong
**File:** `stdlib/consts.tasm`

**Problem:** The value is `-21247483648` and should be `-2147483648`.

### 9.3 🟡 Programs that need `--group-offset` don't say so
**Files:** `example_programs/fib_in_memory.tasm`, `example_programs/list_search.tasm`

**Problem:** `mem_8bit` requires `--group-offset 128` (`mem_14bit` requires 8192), but these examples don't mention it, and the tests compile them without it.

**Potential fixes:** Add a comment to the examples. Better, have the compiler or library enforce it (e.g. a module-level `REQUIRE_OFFSET 128` directive, or automatically offset libraries that need it).

### 9.4 🟡 `list_search.tasm` logic oddities
**File:** `example_programs/list_search.tasm`

**Problem:** `C1` (list start) is set to 20 while the data starts at `C30`. `add C5, C1, C2` computes the end address and is immediately overwritten by `mov C5, 50`. The `found` routine needs `kill` twice (see 1.1).

### 9.5 🟡 Legacy examples no longer compile
**Files:** `example_programs/fetch.tasm`, `example_programs/pointer_test.tasm`, `example_programs/pointer_test1.tasm`

**Problem:** They use the removed `LMALLOC`/`INITMEM`/`LM*`/`MEMREG`/`PTRPOS` instructions. Their tests are commented out, but the files remain in the "valid examples" folder.

**Potential fixes:** Port them to `mem_8bit`, or move them to `old/`.

### 9.6 🟡 Stdlib naming doesn't follow CONTRIBUTING.md
**Files:** `stdlib/*.tasm`, `CONTRIBUTING.md`

**Problem:** CONTRIBUTING requires internal routines to be prefixed `_std_{filename}_`, but the stdlib uses `__name` (e.g. `__ok`, `__wait_until_other`, `__check_bounds`) or no prefix (`bit_8_on`, `temp`). Magic numbers also remain (e.g. `128`, `8200` in `mem_8bit`).

**Potential fixes:** Update either the rule or the code so that they agree.

### 9.7 🟡 Unfinished stdlib modules
**Files:** `stdlib/math.tasm` (`exponent:` has no body; "TODO: implement these in branchless"), `stdlib/compare.tasm` (todo list; only `greater`, `less`, `less_eq`), `stdlib/bits.tasm` (todo: shifts), `stdlib/vec.tasm` ("not verified to work")

**Problem:** An empty `exponent:` routine hits bug 1.3: any `spawn math::exponent` would spawn the next routine instead.

### 9.8 🔴 `mem_14bit` never handles addresses above 8,192 (verified by reading)
**File:** `stdlib/mem_14bit.tasm:31-32`

**Problem:** The bit-14 check (`ifg bit_14_on, bit_13, temp, 8192`) is commented out, and `mem` jumps straight to the bit-13 check. `bit_14_on` is unreachable, so addresses 8,193–16,384 are never reduced by 8,192, despite the header advertising 1–16,384. There is also a comment at line 38 ("past 9,999") that contradicts the header.

**Potential fixes:** Restore the bit-14 check, or document the real range (1–8,192) and drop `bit_14_on`.

### 9.9 🟠 `bits::slow_shift_left`/`slow_shift_right` shift one time too many
**File:** `stdlib/bits.tasm:40-57`

**Problem:** The entry check passes when `c2 >= 0`, and the loop shifts *before* checking `c3_temp >= 0` again. A shift by `n` performs `n + 1` shifts (a shift by 0 shifts once). The comment above `slow_shift_left` also says "right". Both loops also pass `ordered:false` to an instant compare, which ignores it (see 3.1).

**Potential fixes:** Decrement before shifting, or loop while `c3_temp > 0`.

### 9.10 🟠 `math::sign` reads the result of an unordered spawn on the next tick
**File:** `stdlib/math.tasm:31-37`

**Problem:** `spawn compare::greater | ordered:false` is immediately followed by `mov c4, c3`. The spawn has a ~1-tick delay, so `greater` and the `mov` probably run on the same tick, which is a race. `stdlib_modulo.tasm` and `fib_in_memory.tasm` wait after similar calls. Plausible from reading the code; not verified in GD.

**Potential fixes:** Add a wait after each spawn, or restructure as a continuation.

### 9.11 🟡 Stdlib documentation mismatches
**Files:** `stdlib/math.tasm`, `stdlib/compare.tasm`, `stdlib/bits.tasm`, `stdlib/vec.tasm`, `stdlib/debug.tasm`

**Problem:**
- `math.tasm`: the `@param g3/g4` lines contradict the code (see 9.1). "TODO: implement these in branchless" sits above `sign`, which is documented as branchless.
- `compare.tasm`: the TODO list is stale. `less_eq` overwrites `C4` without documenting it. `greater`/`less` rely on bug 4.1.
- `bits.tasm`: the operations are only correct for 0/1 inputs, but are documented as bitwise (`c1 & c2`). `nor`/`xnor` have no doc comment.
- `vec.tasm`:
  - It says C4 returns `-2147483648` on an invalid index, but the failure path never writes C4.
  - The `__check_bounds` comment says "C1: length, C2: index", but the remap passes length/index through C9992/C9993 → C2/C3.
  - It uses unaliased IDs 9991–9994.
  - It calls the other module's internal `debug::__fail`.
  - It imports `mem_8bit` without noting the group-offset requirement.
  - Typo "maintainance".
- `debug.tasm`: typo "if tre", and the `rawtrg` payloads are unaliased magic numbers.
- Doc styles differ between files (`@param`/`@returns` vs "returns (c3):" vs prose).
- Typo "offest" in `mem_8bit.tasm:8` and `mem_14bit.tasm:8`. The usage comment in `mem_14bit.tasm:22,25` shows `ordered: false` (with a space), which the parser rejects if copied.

**Potential fixes:** Agree on one doc-comment format (e.g. `@param`, `@returns`, `@clobbers`, `@ticks`, `@requires-offset`) and apply it.

### 9.12 🟡 Programs in `programs/` don't compile
**Files:** `programs/bubble_sort.tasm`, `programs/reverse.tasm`, `programs/nuclear_reactor.tasm` (134 errors)

**Problem:** They use the removed `LMALLOC`/`LFMALLOC`/`INITMEM`/`LM*`/`MEMSIZE` instructions. README calls `programs/` "valid TASM". `nuclear_reactor.tasm` also has stale comments (they refer to T16–T18 and C4/C8/C10, while the code uses T2016–T2018 and C2004–C2011), a truncated comment at line 259, leftover `BREAKPOINT`s, and same-tick races at lines 90-91 and 483-493. `bubble_sort.tasm` has a comment array that doesn't match its data and leftover debug code (`test_routine`). `overflow.tasm` and `sleep_test.tasm` have no header comments. `sine.tasm` has typos ("jsut", "accurarcy").

**Potential fixes:** Port them to `mem_8bit`, or move them to `old/`.

### 9.13 ⚪ Example program comments and minor logic
**Files:** `example_programs/*.tasm`

**Problem:**
- `is_c1_prime.tasm`: the comment "compute c4 = c1 % c3" is wrong (it computes `c3 * floor(c1 / c3)`), every even number including 2 is reported as not prime, and it mixes the `check_factor` alias with raw `c3`.
- `project_euler_1.tasm`: the comment says "while c1 < 1000", but the code compares with 999. The result is still correct.
- `project_euler_2.tasm`: the routine name `add` shadows the instruction name visually. The even-sum check runs before the bound check, so it is only correct because the first term above the bound is odd.
- `proc_control.tasm`: the header describes a spawner loop that doesn't exist (instances come from repeatedly touching `start`). The label is "cont.".
- `rng.tasm`: `8388608` and the output range and precision are unexplained.
- `incrementer.tasm` and `shapes.tasm` have no header notes (`shapes` is a library and needs `--no-entry-point` to compile on its own).

---

## 10. Tests

<!-- ### 10.1 🔴 `cargo test` fails on the current working tree (verified)
**File:** `tests/all_instructions.tasm:19-22`

**Problem:** It uses the removed `ADDM`/`ADDD`/`SUBM`/`SUBD` (see 4.2). -->

### 10.2 🟡 Tests only check whether compilation succeeds or fails
**File:** `rtasm/src/tests.rs`

**Problem:** No test inspects the generated objects: group IDs, flags, modifier direction, positions. Bugs 1.1, 1.2, 1.3 and 4.1 all pass the current suite.

**Potential fixes:** Add snapshot tests of the linker output (`-l`) and of serialized objects for small programs. Add property tests such as "all groups are unique" and "every spawn target exists".

### 10.3 🟡 Coverage gaps
**File:** `rtasm/src/tests.rs`

**Problem:**
- Only `mem_8bit` and `mem_14bit` are tested from the stdlib (not `math`, `bits`, `compare`, `core`, `consts`, `debug` or `vec`).
- `fib_in_memory` is commented out as "deprecated syntax", but it now compiles.
- None of the new example programs (`collatz`, `temperature`, `stopwatch`, ...) are tested.
- Failure tests only assert "is an error", not *which* error code.

**Potential fixes:** Add a test for every file in `stdlib/` and `example_programs/` (or a glob-driven test), and assert the expected error codes.

<!-- ### 10.4 ⚪ Stale and duplicate test infrastructure
**Files:** `rtasm/src/tests.rs`, `rtasm/profile.ps1`

**Problem:** The commented-out benchmark and `example_no_entry_point` macro arm are dead code. `profile.ps1` references a nonexistent `tests/big_malloc.tasm` and passes a bare `-e`. -->

---

## 11. Documentation

### 11.1 🟠 Two "Home" pages and divergent references
**Files:** `docs/index.md`, `docs/updated_docs_v1.md`, `docs/book/*`

**Problem:** Both reference files have `label: Home`. `index.md` still documents:
- the `=` flag syntax and `nospawn`/`onexit`;
- `ADDM`/`SUBM`/`ADDD`/`SUBD`;
- old init-only rules and merge-conflict markers.

The book links to `../index.md` anchors that only exist in `updated_docs_v1.md`.

**Potential fixes:** Replace `index.md` with `updated_docs_v1.md` once reviewed, then update it for the uncommitted changes (`ADDM` removal, `IMPORTSTD` fix).

### 11.2 🟠 Docs describe bugs that have been fixed, and vice versa
**Files:** `docs/updated_docs_v1.md` §3.1.2.11, `docs/book/07-modules.md` §7.6, `docs/book/09-debugging.md` §9.5

**Problem:** The `stdlib/stdlib` warning is now outdated (fixed in the working tree). The `ADDM` warnings refer to removed instructions. The book recommends empty routines as "do nothing" targets (chapter 2), which bug 1.3 breaks. The "safe swap" example in `updated_docs_v1.md` §3.1.5.2 still reads and writes the same item on one tick.

### 11.3 🟠 README tutorial uses removed instructions
**File:** `README.md:59-141`

**Problem:**
- The tutorial is built on `LMALLOC`, `INITMEM`, `LMREAD`/`LMWRITE`/`LMFUNC`/`LMPTR`/`LMOV`, `MEMREG`, `PTRPOS` and `MEMSIZE`, none of which exist. It calls them "deprecated" when they are removed.
- README says the compiler "is not a standalone executable, and must be executed from source", which contradicts the releases.
- It only mentions the Windows installer.
- The compile steps for the executable and from source are duplicated.

**Potential fixes:** Replace the tutorial with a link to `docs/book/`, or port it to `mem_8bit`.

### 11.4 ⚪ Retype config placeholders
**File:** `docs/retype.yml`

**Problem:** `url: 127.0.0.1` is a placeholder; README says the site is `https://tasm.mntpoint.org/docs/`.

---

## 12. Install scripts, CI and tooling

### 12.1 🔴 Installers remove the existing install before validating input or downloading
**Files:** `install/windows.ps1:51-60, 56-107`, `install/linux.sh:50-60, 80-96`

**Problem:** The "replace existing install?" step runs before the version argument is checked. Running `install` with no version deletes TASM and then prints "Specify a version". A failed download or bad archive also leaves no installation.

**Potential fixes:** Validate arguments, then download and extract to a temp directory, then swap.

### 12.2 🟠 Installer UX and robustness
**Files:** `install/windows.ps1`, `install/linux.sh`

**Problem:**
- Interactive `Read-Host`/`read -rp` prompts break piped or non-interactive installs. Add `-Toolchain`/`--toolchain` and `-Force`/`--yes`.
- The version must include the `v` prefix (`install 0.3.2` gives a 404), even though the stdlib check already strips it. Normalize it once.
- Linux installs into `~/.local/bin/tasm/` (a directory inside `bin`). `~/.local/share/tasm`, with a `tasmc` symlink in `~/.local/bin`, is conventional. Changing this also requires updating `stdlib_dir()` (2.16).
- Linux: no `set -euo pipefail`, no `curl` check, fixed `/tmp/tasm-$version` paths (use `mktemp -d`), a duplicate `mkdir`, and no fish/login-shell handling for PATH. `sed -i "\|$TASMDIR|d"` can delete unrelated lines.
- Windows: PATH writes can leave a leading `;` and flatten `%VAR%` entries. PATH cleanup is skipped if the directory was deleted manually, and uses an exact string match. Temp files leak on error paths. Typo "toolchiain".
- `install/linux.sh` is committed without the executable bit (`100644`), so `./install.sh` from a release archive fails.

### 12.3 🟠 CI gaps
**Files:** `.github/workflows/pr.yml`, `.github/workflows/release.yml`

**Problem:**
- `pr.yml` only runs for `master`, but development happens on `dev`. There is no `cargo fmt --check` or `clippy` (CONTRIBUTING requires `cargo fmt`), and no caching.
- `release.yml`:
  - It has a leftover debug step (`pwd; ls -R`).
  - It builds `windows-gnu` on Ubuntu without configuring the MinGW linker (`CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER`), so that build will likely fail (not verified).
  - It uses `softprops/action-gh-release@v1` (deprecated) and the unpinned `dtolnay/rust-toolchain@master`.
  - There is no test gate before publishing.
  - glibc builds are made on `ubuntu-latest`, which ties them to a new glibc.
  - The musl build includes `cli-clipboard`, which may not link (not verified).

### 12.4 🟡 `.gitattributes` and `.gitignore`
**Files:** `.gitattributes`, `.gitignore`

**Problem:** Shell scripts aren't forced to LF (`*.sh text eol=lf`), so `linux.sh` can get CRLF on Windows checkouts. `.gitignore` still lists `pytasm/namespace.json`.

### 12.5 🟡 `scripts/stdlib_memory.py` no longer matches the stdlib
**File:** `scripts/stdlib_memory.py`

**Problem:**
- It emits `_std_mem_temp`, but `mem_8bit` uses `temp`.
- It only generates bits 8..2 (not `bit_1`).
- It claims 14-bit support but has no 14-bit mode.
- It appends to `bits.txt` on every run without closing the file.
- Template substitution via chained `.replace("M"/"N"/...)` is fragile.

### 12.6 🟡 VS Code extension is early-stage
**Files:** `vscode-ext/package.json`, `vscode-ext/README.md`, `vscode-ext/syntaxes/tasm.tmLanguage.json`

**Problem:**
- `package.json`: `name` is `TASM` (Marketplace names must be lowercase), and it lacks `publisher`, `repository`, `license` and `icon`.
- README is a placeholder ("Features: none yet").
- The grammar is missing `BREAKPOINT` and still lists the removed `ADDM`/`SUBM`/`ADDD`/`SUBD` (see 4.2).
- `language-configuration.json` has a never-matching `decreaseIndentPattern`.
