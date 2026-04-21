#!/bin/sh
# Build a release tarball for hordora.
# Usage: ./release.sh
# Produces: hordora-<version>-x86_64-linux.tar.gz

set -e

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
ARCHIVE="hordora-${VERSION}-x86_64-linux.tar.gz"
STAGING="hordora-${VERSION}"

cargo build --release

rm -rf "$STAGING"
mkdir -p "$STAGING/wallpapers"

cp target/release/hordora "$STAGING/"
cp resources/hordora-session "$STAGING/"
cp resources/hordora.desktop "$STAGING/"
cp resources/hordora-portals.conf "$STAGING/"
cp config.example.toml "$STAGING/config.toml"
cp extras/wallpapers/*.glsl "$STAGING/wallpapers/"

tar czf "$ARCHIVE" "$STAGING"
rm -rf "$STAGING"

echo "Built $ARCHIVE ($(du -h "$ARCHIVE" | cut -f1))"
