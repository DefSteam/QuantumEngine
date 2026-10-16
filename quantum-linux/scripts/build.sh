#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
PACKAGING_DIR="$PROJECT_DIR/packaging"

usage() {
  cat <<EOF
Usage: $0 [--no-bump | version]

Build a pacman package. By default, the patch number is incremented
automatically. Pass --no-bump to keep the current version, or pass a version
such as 0.4.3 to choose one explicitly. After packaging, the script offers to
install the package with pacman.
EOF
}

no_bump=false
requested_version=""
for arg in "$@"; do
  case "$arg" in
    -h|--help)
      usage
      exit 0
      ;;
    --no-bump)
      no_bump=true
      ;;
    -* )
      printf 'Unknown option: %s\n' "$arg" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [[ -n "$requested_version" ]]; then
        usage >&2
        exit 2
      fi
      requested_version="$arg"
      ;;
  esac
done
if [[ "$no_bump" == true && -n "$requested_version" ]]; then
  printf 'Choose either --no-bump or an explicit version.\n' >&2
  exit 2
fi
if (( EUID == 0 )); then
  printf 'Run this script as a regular user; makepkg does not run as root.\n' >&2
  exit 1
fi

for command in awk cargo makepkg npm; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'Required command not found: %s\n' "$command" >&2
    exit 1
  fi
done

current_version="$(awk '
  /^\[workspace\.package\]$/ { in_workspace = 1; next }
  /^\[/ { in_workspace = 0 }
  in_workspace && /^version[[:space:]]*=/ { gsub(/"/, "", $3); print $3; exit }
' "$PROJECT_DIR/Cargo.toml")"
if [[ ! "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  printf 'Could not read a MAJOR.MINOR.PATCH version from Cargo.toml.\n' >&2
  exit 1
fi
if [[ "$no_bump" == true ]]; then
  version="$current_version"
  printf 'Keeping version %s.\n' "$version"
elif [[ -n "$requested_version" ]]; then
  version="$requested_version"
  printf 'Bumping version %s -> %s…\n' "$current_version" "$version"
  "$SCRIPT_DIR/bump-version.sh" "$version"
else
  IFS=. read -r major minor patch <<< "$current_version"
  version="$major.$minor.$((patch + 1))"
  printf 'Bumping version %s -> %s…\n' "$current_version" "$version"
  "$SCRIPT_DIR/bump-version.sh" "$version"
fi

printf '\nInstalling UI dependencies…\n'
(cd "$PROJECT_DIR/ui" && npm ci)

printf '\nBuilding UI assets…\n'
(cd "$PROJECT_DIR/ui" && npm run build)

printf '\nBuilding QuantumEngine release binaries…\n'
cd "$PROJECT_DIR"
cargo build --locked --release --workspace --features quantumengine/custom-protocol

printf '\nPackaging for pacman…\n'
cd "$PACKAGING_DIR"
makepkg --force --nodeps --noconfirm -p PKGBUILD.local

mapfile -t packages < <(makepkg --packagelist -p PKGBUILD.local)
if (( ${#packages[@]} != 1 )); then
  printf 'Expected one package from makepkg; found %d.\n' "${#packages[@]}" >&2
  exit 1
fi
package="${packages[0]}"
if [[ "$package" != /* ]]; then
  package="$PACKAGING_DIR/$package"
fi
if [[ ! -f "$package" ]]; then
  printf 'makepkg reported a package that was not found: %s\n' "$package" >&2
  exit 1
fi

printf '\nPackage ready: %s\n' "$package"
read -r -p 'Install it now with pacman? [y/N] ' answer
case "$answer" in
  [yY]|[yY][eE][sS])
    if ! command -v sudo >/dev/null 2>&1; then
      printf 'sudo was not found; install the package manually with pacman.\n' >&2
      exit 1
    fi
    sudo pacman -U -- "$package"
    ;;
  *)
    printf 'Leaving the package at %s\n' "$package"
    ;;
esac
