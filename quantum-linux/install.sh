#!/usr/bin/env bash
set -Eeuo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

if (($# > 1)); then
  printf 'Usage: %s [package-file.pkg.tar.zst]\n' "$0" >&2
  exit 2
fi

if (($# == 1)); then
  package="$1"
else
  version="$(awk '/^\[workspace\.package\]/{in_package=1; next} /^\[/{in_package=0} in_package && /^version =/{gsub(/"/, "", $3); print $3; exit}' "$PROJECT_DIR/Cargo.toml")"
  shopt -s nullglob
  packages=("$PROJECT_DIR"/packaging/quantumengine-"${version}"-1-"$(uname -m)".pkg.tar.*)
  if ((${#packages[@]} != 1)); then
    printf 'Expected one package for version %s in %s/packaging; found %d. Run ./build.sh first.\n' \
      "$version" "$PROJECT_DIR" "${#packages[@]}" >&2
    exit 1
  fi
  package="${packages[0]}"
fi

if [[ ! -f "$package" ]]; then
  printf 'Package archive not found: %s\n' "$package" >&2
  exit 1
fi

if ! command -v pacman >/dev/null 2>&1; then
  printf 'pacman was not found; this installer is for Arch-based systems.\n' >&2
  exit 1
fi

exec sudo pacman -U -- "$package"
