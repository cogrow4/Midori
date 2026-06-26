#!/usr/bin/env bash
set -euo pipefail

MIDORI_REPO="https://github.com/cogrow4/Midori"
PREFIX="${MIDORI_PREFIX:-$HOME/.local}"
BINDIR="$PREFIX/bin"
VERSION="0.1.0"
PROG="midori"

usage() {
    cat <<EOF
Midori v$VERSION — Installer

Usage:
  install.sh              Build and install midori
  install.sh --uninstall  Remove midori
  install.sh --prefix     Show install prefix
  install.sh --help       Show this help

Environment:
  MIDORI_PREFIX   Install prefix (default: $HOME/.local)
  MIDORI_KEEP_C   Preserve generated .c files during compilation
EOF
}

uninstall() {
    echo "==> Uninstalling Midori..."
    rm -f "$BINDIR/$PROG"
    rm -f "$BINDIR/midori-installed-version"
    echo "==> Removed $BINDIR/$PROG"
    echo "    To remove source files, delete the project directory."
}

show_prefix() {
    echo "$BINDIR"
}

# --- Parse flags ---
case "${1:-}" in
    --help|-h) usage; exit 0 ;;
    --uninstall|-u) uninstall; exit 0 ;;
    --prefix|-p) show_prefix; exit 0 ;;
esac

# --- Install ---
echo "==> Midori v$VERSION Installer"
echo "    Prefix: $PREFIX"

# Check prerequisites
if ! command -v rustc &>/dev/null; then
    echo "error: Rust compiler (rustc) not found. Install from https://rustup.rs"
    exit 1
fi

if ! command -v cc &>/dev/null; then
    echo "error: C compiler (cc) not found. Install Xcode Command Line Tools or GCC."
    exit 1
fi

# Build
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
echo "==> Building midori..."
(cd "$SCRIPT_DIR/compiler" && cargo build --release 2>&1)

# Install
mkdir -p "$BINDIR"
cp "$SCRIPT_DIR/compiler/target/release/$PROG" "$BINDIR/$PROG"
echo "$VERSION" > "$BINDIR/midori-installed-version"

echo "==> Installed $PROG to $BINDIR/$PROG"
echo ""
echo "    Make sure $BINDIR is in your PATH:"
echo "    export PATH=\"\$PATH:$BINDIR\""
echo ""
echo "    Quick test:"
echo "    midori version"
echo "    midori help"
