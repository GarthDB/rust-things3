#!/bin/bash

# Script to update the Homebrew formula with the correct SHA256
# Usage: ./scripts/update-homebrew-formula.sh <version>

set -e

VERSION=${1:-"0.2.0"}
TARBALL_URL="https://github.com/GarthDB/rust-things3/archive/v${VERSION}.tar.gz"

echo "Downloading tarball for version ${VERSION}..."
curl -L -o "/tmp/rust-things3-${VERSION}.tar.gz" "${TARBALL_URL}"

echo "Calculating SHA256..."
if command -v sha256sum >/dev/null 2>&1; then
    SHA256=$(sha256sum "/tmp/rust-things3-${VERSION}.tar.gz" | cut -d' ' -f1)
elif command -v shasum >/dev/null 2>&1; then
    SHA256=$(shasum -a 256 "/tmp/rust-things3-${VERSION}.tar.gz" | cut -d' ' -f1)
else
    echo "Error: Neither sha256sum nor shasum found"
    exit 1
fi

echo "SHA256: ${SHA256}"

# Update the formula file
sed -i.bak "s/sha256 \".*\"/sha256 \"${SHA256}\"/" Formula/things3-cli.rb
sed -i.bak "s|url \".*\"|url \"${TARBALL_URL}\"|" Formula/things3-cli.rb

echo "Updated Formula/things3-cli.rb with SHA256: ${SHA256}"
echo "Please review the changes and commit them."

# Clean up
rm "/tmp/rust-things3-${VERSION}.tar.gz"
rm Formula/things3-cli.rb.bak

echo ""
echo "To update your existing Homebrew tap:"
echo "1. Copy Formula/things3-cli.rb to your homebrew-tap repository"
echo "2. Commit and push the changes"
echo "3. Users can install with: brew install GarthDB/tap/things3-cli"
echo ""
echo "Commands to update your tap:"
echo "  cp Formula/things3-cli.rb /path/to/homebrew-tap/Formula/"
echo "  cd /path/to/homebrew-tap"
echo "  git add Formula/things3-cli.rb"
echo "  git commit -m 'Add/update things3-cli formula v${VERSION}'"
echo "  git push origin main"
