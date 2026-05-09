#!/usr/bin/env bash

APPNAME="tasm"
TASMDIR="$HOME/.local/bin/$APPNAME"

HELP=$(cat <<'EOF'
This is the install script for the TASM language.
One of these actions must be specified as the first argument:
  - "remove": remove the current installed version of tasm if it exists
  - "install <version>": install the specified version. overwrites (with permission) any other version
EOF
)

INSTALLPROMPT=$(cat <<'EOF'
Which version would you like to install?
[0] x86_64-unknown-linux-gnu
[1] x86_64-unknown-linux-musl (static, no glibc dependency)
EOF
)

install() {
    local version="$1"

    # check for existing install
    if [ -d "$TASMDIR" ]; then
        echo -e "\033[33mDetected an already installed version of tasm\033[0m"

        # try to print installed version (silently fail if broken)
        "$TASMDIR/tasmc" --version 2>/dev/null || true

        read -rp "> Confirm removing this version to install $APPNAME $version (y/n): " confirm
        if [ "$confirm" != "y" ]; then
            return
        fi

        remove
    fi

    if [ -z "$version" ]; then
        echo "Specify a version: ./install.sh install <version>"
        return
    fi

    echo "$INSTALLPROMPT"
    read -rp "> Option number: " choice
    case "$choice" in
        0) triple="x86_64-unknown-linux-gnu" ;;
        1) triple="x86_64-unknown-linux-musl" ;;
        *)
            echo "Invalid option. Aborting..."
            return
            ;;
    esac

    local url="https://github.com/ArrowSlashArrow/tasm-lang/releases/download/$version/tasm-$version-$triple.tar.gz"
    local outdir="/tmp/tasm-$version"
    local outfile="/tmp/tasm-$version.tar.gz"

    echo "Downloading $url..."
    if ! curl -fsSL "$url" -o "$outfile"; then
        echo "Unable to download file."
        return
    fi

    # remove stale temp dir
    rm -rf "$outdir"

    mkdir -p "$outdir"
    tar -xzf "$outfile" -C "$outdir"

    mkdir -p "$TASMDIR"
    cp -r "$outdir"/. "$TASMDIR/"
    chmod +x "$TASMDIR/tasmc"

    # add to PATH in shell config if not already present
    local shell_rc
    case "$SHELL" in
        */zsh)  shell_rc="$HOME/.zshrc" ;;
        */bash) shell_rc="$HOME/.bashrc" ;;
        *)      shell_rc="$HOME/.profile" ;;
    esac

    if ! grep -qF "$TASMDIR" "$shell_rc" 2>/dev/null; then
        echo "export PATH=\"\$PATH:$TASMDIR\"" >> "$shell_rc"
        echo "Added executable path to PATH (in $shell_rc)"
    else
        echo "PATH entry already exists in $shell_rc, skipping."
    fi

    rm -rf "$outdir" "$outfile"

    echo "$APPNAME $version [$triple] has been installed."
    echo "Restart your shell or run source ~/.bashrc for PATH changes to take effect."
}

remove() {
    if [ -d "$TASMDIR" ]; then
        rm -rf "$TASMDIR"

        # remove PATH entry from shell configs
        for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
            if [ -f "$rc" ]; then
                # remove any line that exports or appends TASMDIR to PATH
                sed -i "\|$TASMDIR|d" "$rc"
            fi
        done

        echo -e "\033[32m$APPNAME was removed.\033[0m"
    else
        echo "$APPNAME is already removed."
    fi
}

case "$1" in
    install) install "$2" ;;
    remove)  remove ;;
    *)       echo "$HELP" ;;
esac