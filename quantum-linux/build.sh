#!/usr/bin/env bash
set -Eeuo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

for command in cargo npm makepkg; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'Required command not found: %s\n' "$command" >&2
    exit 1
  fi
done

printf 'Installing UI dependencies…\n'
npm --prefix "$PROJECT_DIR/ui" ci

printf 'Building UI assets…\n'
npm --prefix "$PROJECT_DIR/ui" run build

printf 'Building QuantumEngine (release)…\n'
cd "$PROJECT_DIR"
cargo build --locked --release --workspace --features quantumengine/custom-protocol

printf 'Packaging local build for pacman…\n'
cd "$PROJECT_DIR/packaging"
makepkg --force --nodeps -p PKGBUILD.local

printf '\nBuild complete. Binaries are in %s/target/release/\n' "$PROJECT_DIR"
printf 'Package archive is in %s/packaging/\n' "$PROJECT_DIR"
