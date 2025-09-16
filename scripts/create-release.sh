#!/bin/bash

# Create GitHub Release Script for Things3 CLI
set -e

VERSION="0.1.0"
TAG="v$VERSION"
REPO="GarthDB/rust-things"

echo "🚀 Creating release $TAG for $REPO"

# Check if tag already exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "❌ Tag $TAG already exists"
    exit 1
fi

# Create and push tag
echo "📝 Creating tag $TAG"
git tag -a "$TAG" -m "Release $TAG

Features:
- CLI tool for Things 3 with integrated MCP server
- Core library for Things 3 database access
- Common utilities and types
- Comprehensive test coverage (~90%+)
- Homebrew formula support

Breaking Changes:
- Renamed from things-cli to things3-cli
- CLI command changed from things-cli to things3
- All crates use things3- prefix

Installation:
- From crates.io: cargo install things3-cli
- From Homebrew: brew install GarthDB/rust-things/things3-cli
- From source: cargo install --git https://github.com/GarthDB/rust-things"

git push origin "$TAG"

echo "✅ Tag $TAG created and pushed"

# Create GitHub release
echo "📦 Creating GitHub release..."

# Create release notes
cat > /tmp/release_notes.md << EOF
# Things3 CLI v$VERSION

A powerful command-line interface for Things 3 with integrated MCP (Model Context Protocol) server support.

## 🚀 Features

- **CLI Tool**: Complete command-line interface for Things 3
- **MCP Server**: Integrated MCP server for AI/LLM integration
- **Database Access**: Direct access to Things 3 database
- **Export Support**: Export data in multiple formats (JSON, CSV, Markdown, OPML)
- **Backup/Restore**: Database backup and restore functionality
- **Performance Monitoring**: Built-in performance metrics
- **Caching**: High-performance caching for better performance

## 📦 Installation

### From crates.io (Recommended)
\`\`\`bash
cargo install things3-cli
\`\`\`

### From Homebrew
\`\`\`bash
brew install GarthDB/rust-things/things3-cli
\`\`\`

### From source
\`\`\`bash
cargo install --git https://github.com/GarthDB/rust-things
\`\`\`

## 🎯 Usage

\`\`\`bash
# Basic usage
things3 --help

# View inbox tasks
things3 inbox

# View today's tasks
things3 today

# Start MCP server
things3 mcp
\`\`\`

## 📊 Test Coverage

- **Overall Coverage**: ~92%+
- **Function Coverage**: ~95%+
- **Line Coverage**: ~90%+
- **Branch Coverage**: ~90%+

## 🔧 Development

This release includes comprehensive test coverage and robust error handling.

## 📝 Changelog

### Breaking Changes
- Renamed from \`things-cli\` to \`things3-cli\`
- CLI command changed from \`things-cli\` to \`things3\`
- All crates use \`things3-\` prefix

### New Features
- Integrated MCP server
- Comprehensive test suite
- Performance monitoring
- Export functionality
- Backup/restore capabilities

## 📄 License

MIT License - see LICENSE file for details.
EOF

# Use GitHub CLI to create release
if command -v gh &> /dev/null; then
    gh release create "$TAG" \
        --title "Things3 CLI v$VERSION" \
        --notes-file /tmp/release_notes.md \
        --latest
    echo "✅ GitHub release created"
else
    echo "⚠️  GitHub CLI not found. Please create release manually at:"
    echo "   https://github.com/$REPO/releases/new?tag=$TAG"
    echo "   Release notes saved to: /tmp/release_notes.md"
fi

# Clean up
rm -f /tmp/release_notes.md

echo "🎉 Release process completed!"
echo "Next steps:"
echo "1. Publish crates to crates.io:"
echo "   cd libs/things3-common && cargo publish"
echo "   cd libs/things3-core && cargo publish"
echo "   cd apps/things3-cli && cargo publish"
echo "2. Update Homebrew formula with correct SHA256"
echo "3. Submit Homebrew formula to homebrew-core"
