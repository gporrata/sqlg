#!/bin/sh
# Copy the sqlg CLI next to the Tauri sidecar path for the current host triple.
set -eu
mode="${1:-debug}"
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
if [ "$mode" = "release" ]; then
  cargo build -p sqlg-cli --release --manifest-path "$root/Cargo.toml"
  src="$root/target/release/sqlg"
else
  cargo build -p sqlg-cli --manifest-path "$root/Cargo.toml"
  src="$root/target/debug/sqlg"
fi
triple=$(rustc -vV | sed -n 's/^host: //p')
mkdir -p "$root/src-tauri/binaries"
cp "$src" "$root/src-tauri/binaries/sqlg-$triple"
chmod +x "$root/src-tauri/binaries/sqlg-$triple"
echo "staged $root/src-tauri/binaries/sqlg-$triple"
