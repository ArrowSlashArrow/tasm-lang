// Starts the TASM language server (tasm-lsp, see ../../tasm-lsp) and connects it to VS Code,
// and adds commands that compile the current file with tasmc.
const { execFile } = require("child_process");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

const EXE = process.platform === "win32" ? "tasm-lsp.exe" : "tasm-lsp";

/** @type {LanguageClient | undefined} */
let client;

/**
 * Server lookup order: the `tasm.server.path` setting, a binary bundled in `server/`,
 * a build in the repository's `tasm-lsp/target/` (when running from source), then PATH.
 */
function findServer(context) {
    const configured = vscode.workspace.getConfiguration("tasm").get("server.path");
    if (configured) {
        return configured;
    }
    const candidates = [
        path.join(context.extensionPath, "server", EXE),
        path.join(context.extensionPath, "..", "tasm-lsp", "target", "release", EXE),
        path.join(context.extensionPath, "..", "tasm-lsp", "target", "debug", EXE),
    ];
    const found = candidates.find((c) => fs.existsSync(c));
    if (found && process.platform !== "win32") {
        // the release .vsix bundles the server; make sure it kept its executable bit
        try {
            fs.accessSync(found, fs.constants.X_OK);
        } catch {
            fs.chmodSync(found, 0o755);
        }
    }
    return found ?? "tasm-lsp";
}

async function start(context) {
    const config = vscode.workspace.getConfiguration("tasm");
    const command = findServer(context);
    client = new LanguageClient(
        "tasm",
        "TASM Language Server",
        { run: { command }, debug: { command } },
        {
            documentSelector: [{ scheme: "file", language: "tasm" }],
            initializationOptions: {
                stdlibPath: config.get("stdlibPath") || null,
                requireEntryPoint: config.get("diagnostics.requireEntryPoint"),
            },
        }
    );
    try {
        await client.start();
    } catch (e) {
        client = undefined;
        vscode.window.showErrorMessage(
            `Could not start the TASM language server (${command}): ${e.message ?? e}. ` +
                "Build it with `cargo build --release` in tasm-lsp/ or set `tasm.server.path`."
        );
    }
}

const TASMC = process.platform === "win32" ? "tasmc.exe" : "tasmc";

/** @type {vscode.OutputChannel | undefined} */
let output;

/**
 * Compiler lookup order: the `tasm.compiler.path` setting, a build in the repository's
 * `rtasm/target/` (when running from source, so that it matches the source), the installer's
 * location (%APPDATA%\tasm or ~/.local/bin/tasm), then PATH.
 */
function findCompiler(context) {
    const configured = vscode.workspace.getConfiguration("tasm").get("compiler.path");
    if (configured) {
        return configured;
    }
    const installDir =
        process.platform === "win32"
            ? path.join(process.env.APPDATA ?? "", "tasm")
            : path.join(process.env.HOME ?? "", ".local", "bin", "tasm");
    const candidates = [
        path.join(context.extensionPath, "..", "rtasm", "target", "release", TASMC),
        path.join(context.extensionPath, "..", "rtasm", "target", "debug", TASMC),
        path.join(installDir, TASMC),
    ];
    return candidates.find((c) => fs.existsSync(c)) ?? "tasmc";
}

/** Each compile command: the tasmc arguments and what to say when it succeeds. */
const COMPILE_COMMANDS = {
    "tasm.compileGmd": { args: ["--gmd"], target: "gmd" },
    "tasm.compileGmdRelease": { args: ["--gmd", "--release"], target: "gmd" },
    "tasm.compileGmdUnorderedStart": { args: ["--gmd", "--unordered-start"], target: "gmd" },
    "tasm.compileClipboard": { args: ["--clipboard"], target: "clipboard" },
    // used by the "Compile to Clipboard" lens above `_start`
    "tasm.compileClipboardLens": { args: ["--no-entry-point", "--clipboard"], target: "clipboard" },
};

/**
 * Compiles a TASM file with tasmc. tasmc runs in the file's directory, so a .gmd is written
 * next to the file (named after it, e.g. `main.tasm.gmd`).
 * @param {vscode.Uri | undefined} uri the file; defaults to the active editor's file
 */
async function compile(context, uri, { args, target }) {
    const doc = uri
        ? await vscode.workspace.openTextDocument(uri)
        : vscode.window.activeTextEditor?.document;
    if (!doc || doc.languageId !== "tasm" || doc.uri.scheme !== "file") {
        vscode.window.showErrorMessage("Open a saved .tasm file to compile it.");
        return;
    }
    if (doc.isDirty && !(await doc.save())) {
        return;
    }

    const file = doc.uri.fsPath;
    const name = path.basename(file);
    const cwd = path.dirname(file);
    const compiler = findCompiler(context);
    output ??= vscode.window.createOutputChannel("TASM");
    output.appendLine(`> ${compiler} ${[name, ...args].join(" ")}  (in ${cwd})`);

    const { error, text } = await new Promise((resolve) => {
        execFile(compiler, [name, ...args], { cwd }, (error, stdout, stderr) =>
            resolve({ error, text: `${stdout}${stderr}` })
        );
    });
    output.appendLine(text.trimEnd());
    output.appendLine("");

    // tasmc reports compile errors on stdout and still exits with 0
    const failed = error || /Unable to|Failed to|\[E\d{4}\]/.test(text);
    if (failed) {
        const reason = error?.code === "ENOENT"
            ? `Could not run ${compiler}. Install TASM or set \`tasm.compiler.path\`.`
            : `Compiling ${name} failed.`;
        const choice = await vscode.window.showErrorMessage(reason, "Show Output");
        if (choice) {
            output.show(true);
        }
        return;
    }
    vscode.window.showInformationMessage(
        target === "gmd"
            ? `Compiled ${name} to ${path.join(cwd, `${name}.gmd`)}.`
            : `Compiled ${name} to the clipboard.`
    );
}

/** Puts a "Compile to Clipboard" lens above `_start:`. */
const compileLens = {
    provideCodeLenses(doc) {
        const lenses = [];
        for (let i = 0; i < doc.lineCount; i++) {
            if (/^_start:/.test(doc.lineAt(i).text)) {
                lenses.push(
                    new vscode.CodeLens(new vscode.Range(i, 0, i, 0), {
                        title: "Compile to Clipboard",
                        command: "tasm.compileClipboardLens",
                        arguments: [doc.uri],
                    })
                );
            }
        }
        return lenses;
    },
};

async function restart(context) {
    if (client) {
        await client.stop();
        client = undefined;
    }
    await start(context);
}

async function activate(context) {
    context.subscriptions.push(
        vscode.commands.registerCommand("tasm.restartServer", () => restart(context)),
        vscode.workspace.onDidChangeConfiguration((e) => {
            // the compiler path is looked up on every compile; only the server needs a restart
            if (e.affectsConfiguration("tasm") && !e.affectsConfiguration("tasm.compiler")) {
                restart(context);
            }
        }),
        vscode.languages.registerCodeLensProvider({ language: "tasm", scheme: "file" }, compileLens),
        ...Object.entries(COMPILE_COMMANDS).map(([id, spec]) =>
            // from the editor title or explorer menus, the file's URI is passed in
            vscode.commands.registerCommand(id, (uri) =>
                compile(context, uri instanceof vscode.Uri ? uri : undefined, spec)
            )
        )
    );
    await start(context);
}

function deactivate() {
    return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
