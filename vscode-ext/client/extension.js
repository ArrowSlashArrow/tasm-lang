// Starts the TASM language server (tasm-lsp, see ../../tasm-lsp) and connects it to VS Code.
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
    return candidates.find((c) => fs.existsSync(c)) ?? "tasm-lsp";
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
            if (e.affectsConfiguration("tasm")) {
                restart(context);
            }
        })
    );
    await start(context);
}

function deactivate() {
    return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
