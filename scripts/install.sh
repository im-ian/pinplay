#!/bin/sh

set -eu

install_dependencies=false

if [ "${1:-}" = "--with-deps" ]; then
  install_dependencies=true
  shift
fi

if [ "$#" -ne 0 ]; then
  echo "usage: ./scripts/install.sh [--with-deps]" >&2
  exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "pinplay: Rust and Cargo are required: https://rustup.rs" >&2
  exit 1
fi

if [ "$install_dependencies" = true ]; then
  if ! command -v brew >/dev/null 2>&1; then
    echo "pinplay: --with-deps currently requires Homebrew" >&2
    exit 1
  fi
  brew install mpv yt-dlp
fi

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_directory=$(dirname -- "$script_directory")

if [ -n "${PINPLAY_INSTALL_DIR:-}" ]; then
  install_directory=$PINPLAY_INSTALL_DIR
elif command -v brew >/dev/null 2>&1; then
  install_directory=$(brew --prefix)/bin
else
  install_directory=${XDG_BIN_HOME:-$HOME/.local/bin}
fi

cd "$project_directory"
cargo build --release --locked
mkdir -p "$install_directory"
install -m 755 target/release/pinplay "$install_directory/pinplay"

echo "Installed pinplay to $install_directory/pinplay"
echo "Run: pinplay doctor"

