#!/bin/sh
# Crew installer — detects OS/arch and installs the latest release from GitHub.
#
# Usage:
#   curl -sSfL https://raw.githubusercontent.com/ashishtyagi10/crew/main/install.sh | sh
#
set -e

REPO="ashishtyagi10/crew"
BIN_NAME="crew"

# Always install to ~/.local/bin so neither install nor `/update` ever
# needs sudo. The user can override with INSTALL_DIR=/path env var if
# they really want a system location.
if [ -n "${INSTALL_DIR:-}" ]; then
    : # user override via env
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR" || {
        echo "Error: cannot create ${INSTALL_DIR}." >&2
        echo "Set INSTALL_DIR=/some/writable/dir and re-run." >&2
        exit 1
    }
fi

main() {
    need_cmd curl
    need_cmd tar

    os="$(detect_os)"
    arch="$(detect_arch)"
    target="${arch}-${os}"

    echo "Detected platform: ${target}"

    # Get the latest release tag
    latest="$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"

    if [ -z "$latest" ]; then
        echo "Error: could not determine latest release." >&2
        exit 1
    fi

    echo "Latest release: ${latest}"

    # Determine archive format
    case "$os" in
        *windows*) ext="zip" ;;
        *)         ext="tar.gz" ;;
    esac

    url="https://github.com/${REPO}/releases/download/${latest}/${BIN_NAME}-${latest}-${target}.${ext}"
    echo "Downloading ${url} ..."

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    curl -sSfL "$url" -o "${tmpdir}/archive.${ext}"

    case "$ext" in
        tar.gz)
            tar xzf "${tmpdir}/archive.${ext}" -C "$tmpdir"
            ;;
        zip)
            need_cmd unzip
            unzip -q "${tmpdir}/archive.${ext}" -d "$tmpdir"
            ;;
    esac

    # Install the binary (no sudo — INSTALL_DIR is user-writable by design)
    if [ -w "$INSTALL_DIR" ]; then
        install -m 755 "${tmpdir}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    else
        echo "Error: ${INSTALL_DIR} is not writable by the current user." >&2
        echo "Re-run with INSTALL_DIR set to a writable directory." >&2
        exit 1
    fi

    echo ""
    echo "Installed ${BIN_NAME} ${latest} to ${INSTALL_DIR}/${BIN_NAME}"

    # Register Crew in the OS app menu (Spotlight / Start menu / .desktop).
    # Best-effort: an older binary without the subcommand must not fail install.
    "${INSTALL_DIR}/${BIN_NAME}" install-app 2>/dev/null || true

    # Check if INSTALL_DIR is in PATH
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo ""
            echo "NOTE: ${INSTALL_DIR} is not in your PATH."
            echo "Add it by running:"
            echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.$(basename "$SHELL")rc"
            echo "  source ~/.$(basename "$SHELL")rc"
            ;;
    esac

    echo ""
    echo "Run 'crew' to start. Type '/update' inside crew to install new releases."
}

detect_os() {
    uname_s="$(uname -s)"
    case "$uname_s" in
        Linux*)  echo "unknown-linux-gnu" ;;
        Darwin*) echo "apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*) echo "pc-windows-msvc" ;;
        *)
            echo "Unsupported OS: ${uname_s}" >&2
            exit 1
            ;;
    esac
}

detect_arch() {
    uname_m="$(uname -m)"
    case "$uname_m" in
        x86_64|amd64)  echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *)
            echo "Unsupported architecture: ${uname_m}" >&2
            exit 1
            ;;
    esac
}

need_cmd() {
    if ! command -v "$1" > /dev/null 2>&1; then
        echo "Error: '${1}' is required but not found." >&2
        exit 1
    fi
}

main "$@"
