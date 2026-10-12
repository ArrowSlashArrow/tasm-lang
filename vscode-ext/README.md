# vscode extension for TASM
Support for TASM (Trigger Assembly)

## Features
- Syntax highlighting and snippets
- Language server ([`tasm-lsp`](../tasm-lsp)):
  - diagnostics from the compiler's lexer, plus link checks (`module::symbol`, imports, circular imports) and per-instruction checks (e.g. negative waits, writing to `ATTEMPTS`)
  - completion for instructions, arguments (filtered by the instruction's argsets), local and imported aliases/routines, `IMPORT`/`IMPORTSTD` paths, flags and `remap` entries
  - hover for instructions, arguments, flags and imports; routines show their doc comment
  - go to definition (ctrl + click) for imports, routines and aliases, including `module::symbol` in other files
  - suggestions from modules that aren't imported yet (stdlib, and `.tasm` files in the file's directory and its subdirectories) once 2 characters of a name are typed; choosing one adds the `import`/`importstd` to `_init`
- Compile commands (command palette, editor and explorer context menus), which run `tasmc` in the file's directory:
  - **TASM: Compile to .gmd**, **(Release Mode)** and **(Unordered Start)**: write `<file>.tasm.gmd` next to the file
  - **TASM: Compile to Clipboard** (Windows only, paste with BetterEdit)
  - a **Compile to Clipboard** lens above `_start:` (runs with `--no-entry-point --clipboard`)

<!-- ## Installing 
Each GitHub release has a `.vsix` per platform (`win32-x64`, `linux-x64`) with the server bundled, built by `.github/workflows/vscode-ext.yml`. Install it with `code --install-extension <file>.vsix`.

## Running from source
1. Build the server: `cargo build --release` in `tasm-lsp/`.
2. Install the client's dependencies: `npm install` in `vscode-ext/`.
3. Open `vscode-ext/` in VS Code and press `F5`.

<!-- The extension looks for the server in this order: the `tasm.server.path` setting, `server/tasm-lsp[.exe]` inside the extension, `../tasm-lsp/target/{release,debug}/`, then `tasm-lsp` on `PATH`. -->

## Settings
- `tasm.server.path`: path to the `tasm-lsp` executable.
- `tasm.compiler.path`: path to the `tasmc` executable used by the compile commands (default: a build in `rtasm/target/` when running from source, the installer's location, then `PATH`).
- `tasm.stdlibPath`: directory used for `IMPORTSTD` (default: the installer's location).
- `tasm.diagnostics.requireEntryPoint`: report E0011 for files without `_start` (off by default, since libraries don't have one).
