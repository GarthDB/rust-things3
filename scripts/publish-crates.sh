#!/bin/bash

# Publish crates to crates.io in dependency order
set -e

echo "🚀 Publishing Things3 crates to crates.io"

# Check if logged in
if ! cargo owner --list >/dev/null 2>&1; then
    echo "❌ Not logged in to crates.io. Please run 'cargo login' first."
    exit 1
fi

echo "✅ Logged in to crates.io"

# 1. Publish things3-common (no dependencies on our crates)
echo "📦 Publishing things3-common..."
cd libs/things3-common
cargo publish
echo "✅ things3-common published successfully"

# 2. Publish things3-core (depends on things3-common)
echo "📦 Publishing things3-core..."
cd ../things3-core
cargo publish
echo "✅ things3-core published successfully"

# 3. Publish things3-cli (depends on both)
echo "📦 Publishing things3-cli..."
cd ../../apps/things3-cli
cargo publish
echo "✅ things3-cli published successfully"

echo "🎉 All crates published successfully!"
echo ""
echo "📋 Published crates:"
echo "  - things3-common: https://crates.io/crates/things3-common"
echo "  - things3-core: https://crates.io/crates/things3-core"
echo "  - things3-cli: https://crates.io/crates/things3-cli"
echo ""
echo "🔗 Installation command:"
echo "  cargo install things3-cli"
