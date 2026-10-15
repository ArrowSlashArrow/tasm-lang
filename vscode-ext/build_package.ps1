cd "$PSScriptRoot/../tasm-lsp"
cargo build --release
cd  $PSScriptRoot
echo "y" | npx @vscode/vsce package
