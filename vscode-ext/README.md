# vscode extension for TASM
Support for TASM

## Features
- Syntax highlighting and snippets
- Language server ([`tasm-lsp`](../tasm-lsp)):
  - diagnostics from the compiler's lexer, plus link checks (`module::symbol`, imports, circular imports) and per-instruction checks (e.g. negative waits, writing to `ATTEMPTS`)
  - completion for instructions, arguments (filtered by the instruction's argsets), local and imported aliases/routines, `IMPORT`/`IMPORTSTD` paths, flags and `remap` entries
  - hover for instructions, arguments, flags and imports; routines show their doc comment
  - go to definition (ctrl + click) for imports, routines and aliases, including `module::symbol` in other files

### Documenting routines
The comment lines directly above a routine (and the comment on its label line) are its documentation, shown when hovering over the routine:

```
; Adds two counters.
; @param c1 first operand
; @param c2 second operand
; @returns c3 the sum
; @0tick
add_two:
    add c3, c1, c2
```

Tags: `@param <name> <description>`, `@returns <name> <description>`, `@0tick` (safe to 0-tick) and `@no0tick <reason>`. A blank `;` line starts a new paragraph.

## Installing
Each GitHub release has a `.vsix` per platform (`win32-x64`, `linux-x64`) with the server bundled, built by `.github/workflows/vscode-ext.yml`. Install it with `code --install-extension <file>.vsix`.

## Running from source
1. Build the server: `cargo build --release` in `tasm-lsp/`.
2. Install the client's dependencies: `npm install` in `vscode-ext/`.
3. Open `vscode-ext/` in VS Code and press `F5`.

The extension looks for the server in this order: the `tasm.server.path` setting, `server/tasm-lsp[.exe]` inside the extension, `../tasm-lsp/target/{release,debug}/`, then `tasm-lsp` on `PATH`.

## Settings
- `tasm.server.path`: path to the `tasm-lsp` executable.
- `tasm.stdlibPath`: directory used for `IMPORTSTD` (default: the installer's location).
- `tasm.diagnostics.requireEntryPoint`: report E0011 for files without `_start` (off by default, since libraries don't have one).
