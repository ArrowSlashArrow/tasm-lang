#!/bin/bash

APPNAME="tasm"
TASMDIR="$HOME/.tasm"

HELP="This is the install script for the TASM language.
One of these actions must be specified as the first argument:
- \"remove\": to remove the current installed version of tasm if it exists
- \"install <version>\": install the specified version. overwrites any other version"

INSTALLPROMPT="Which version would you like to install?
[0] Using GNU toolchain (x86_64-unknown-linux-gnu)
[1] Using MUSL toolchain (x86_64-unknown-linux-musl)"

install_tasm() {
    local version=$1

    # check if there is an installed version already
    if [ -d "$TASMDIR" ]; then
        echo -e "\e[33mDetected an already installed version of tasm\e[0m"
        if [ -f "$TASMDIR/tasmc" ]; then
            "$TASMDIR/tasmc" --version 2>/dev/null || true
        fi

        read -p "> Confirm removing this version to install $APPNAME $version (y/n) " confirm
        if [ "$confirm" != "y" ]; then
            return
        fi

        remove_tasm
    fi

    if [ -z "$version" ]; then
        echo "Specify a version: ./linux.sh install <version>"
        return
    fi

    echo "$INSTALLPROMPT"
    read -p "> Option number: " choice

    local triple=""
    case $choice in
        0) triple="x86_64-unknown-linux-gnu" ;;
        1) triple="x86_64-unknown-linux-musl" ;;
        *) echo "Invalid option. Aborting..."; return ;;
    esac

    # ok now install the new version
    local url="https://github.com/ArrowSlashArrow/tasm-lang/releases/download/$version/tasm-$version-$triple.tar.gz"
    local outdir="/tmp/tasm-$version"
    local outfile_tar="/tmp/tasm-$version.tar.gz"

    if ! curl -fLo "$outfile_tar" "$url"; then
        echo "Unable to download file."
        return
    fi

    # remove stale temp dir
    rm -rf "$outdir"
    mkdir -p "$outdir"
    
    # then update the system
    tar -xzf "$outfile_tar" -C "$outdir"

    mkdir -p "$TASMDIR"
    cp -rf "$outdir"/* "$TASMDIR/"
    chmod +x "$TASMDIR/tasmc"

    # Add to PATH
    local profile_file=""
    if [ -f "$HOME/.bashrc" ]; then
        profile_file="$HOME/.bashrc"
    elif [ -f "$HOME/.zshrc" ]; then
        profile_file="$HOME/.zshrc"
    elif [ -f "$HOME/.profile" ]; then
        profile_file="$HOME/.profile"
    fi

    if [ -n "$profile_file" ]; then
        if ! grep -q "$TASMDIR" "$profile_file"; then
            echo "" >> "$profile_file"
            echo "# TASM language" >> "$profile_file"
            echo "export PATH=\"\$PATH:$TASMDIR\"" >> "$profile_file"
            echo "Added executable path to $profile_file"
        else
            echo "Path already exists in $profile_file"
        fi
    else
        echo "Could not find a profile file (.bashrc, .zshrc, .profile) to update PATH."
        echo "Please add export PATH=\"\$PATH:$TASMDIR\" to your shell profile."
    fi

    rm -rf "$outdir" "$outfile_tar"

    echo "$APPNAME $version [$triple] has been installed."
    echo "Restart your shell for PATH changes to take effect."
}

remove_tasm() {
    if [ -d "$TASMDIR" ]; then
        # remove install dir
        rm -rf "$TASMDIR"
        echo -e "\e[32m$APPNAME was removed.\e[0m"
        echo "Note: You may need to manually remove the $TASMDIR path from your shell profile (e.g. .bashrc or .zshrc)"
    else
        echo "$APPNAME is already removed."
    fi
}

action=$1
version=$2

case $action in
    "install") install_tasm "$version" ;;
    "remove") remove_tasm ;;
    *) echo "$HELP" ;;
esac