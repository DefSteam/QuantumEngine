#!/bin/sh
# Sets the version everywhere it is written down and refreshes Cargo.lock.
# Then commit, tag and push; the release workflow does the rest.
set -eu

if [ $# -ne 1 ]; then
    echo "usage: $0 <version>, e.g. $0 0.3.0" >&2
    exit 2
fi
version=$1
if ! printf '%s' "$version" | grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+'; then
    echo "not a version: $version (want MAJOR.MINOR.PATCH)" >&2
    exit 2
fi

cd "$(dirname "$0")/.."
sed -i -E "s/^version = \"[^\"]*\"/version = \"$version\"/" Cargo.toml
sed -i -E "s/(path = \"crates\/[a-z-]+\", version = )\"[^\"]*\"/\1\"$version\"/" Cargo.toml
sed -i -E "s/^pkgver=.*/pkgver=$version/; s/^pkgrel=.*/pkgrel=1/" packaging/PKGBUILD
sed -i -E "s/(\"Version\": )\"[^\"]*\"/\1\"$version\"/" packaging/plasmoid/org.quantumengine.plasmoid/metadata.json
cargo update --workspace --quiet

echo "Version set to $version. Commit it, then:"
echo "  git tag -a v$version -m \"v$version\" && git push --follow-tags"
