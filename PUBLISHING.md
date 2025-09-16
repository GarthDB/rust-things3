# Publishing Guide

This guide covers how to publish the Things3 CLI crates to crates.io and set up Homebrew distribution.

## Prerequisites

1. **crates.io Account**: Sign up at https://crates.io and get an API token
2. **GitHub Account**: For releases and Homebrew tap
3. **Homebrew**: For testing the formula locally

## Step 1: Login to crates.io

```bash
cargo login
# Enter your API token when prompted
```

## Step 2: Publish Crates (in order)

Publish dependencies first:

```bash
# 1. Publish common library
cd libs/things3-common
cargo publish

# 2. Publish core library
cd ../things3-core
cargo publish

# 3. Publish CLI
cd ../../apps/things3-cli
cargo publish
```

## Step 3: Create GitHub Release

```bash
# Run the release script
./scripts/create-release.sh
```

Or manually:

```bash
# Create and push tag
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0

# Create GitHub release at:
# https://github.com/GarthDB/rust-things/releases/new?tag=v0.1.0
```

## Step 4: Update Homebrew Formula

After creating the release, update the SHA256:

```bash
# Get the SHA256 of the release tarball
curl -L https://github.com/GarthDB/rust-things/archive/v0.1.0.tar.gz | shasum -a 256

# Update the formula
# Edit homebrew-tap/Formula/things3-cli.rb and replace PLACEHOLDER_SHA256
```

## Step 5: Test Homebrew Formula Locally

```bash
# Test the formula
brew install --build-from-source ./homebrew-tap/Formula/things3-cli.rb

# Test the installation
things3 --help
```

## Step 6: Set up Homebrew Tap

### Option A: Personal Tap (Recommended for now)

1. Create a new repository: `GarthDB/homebrew-tap`
2. Copy the formula there
3. Users can install with:
   ```bash
   brew install GarthDB/tap/things3-cli
   ```

### Option B: Submit to homebrew-core

1. Fork homebrew-core
2. Create a new formula file
3. Submit a pull request

## Step 7: Update Documentation

Update the README with installation instructions:

```markdown
## Installation

### From crates.io (Recommended)
```bash
cargo install things3-cli
```

### From Homebrew
```bash
brew install GarthDB/tap/things3-cli
```

### From source
```bash
cargo install --git https://github.com/GarthDB/rust-things
```
```

## Verification

After publishing, verify everything works:

```bash
# Test crates.io installation
cargo install things3-cli
things3 --help

# Test Homebrew installation
brew install GarthDB/tap/things3-cli
things3 --help
```

## Troubleshooting

### Common Issues

1. **Crate already exists**: Wait 24 hours or use a different version
2. **Missing dependencies**: Ensure all dependencies are published first
3. **Homebrew formula fails**: Check the SHA256 and URL
4. **Permission denied**: Ensure you're logged in to crates.io

### Version Bumping

For future releases:

1. Update version in `Cargo.toml` files
2. Update Homebrew formula version and SHA256
3. Create new GitHub release
4. Publish crates in dependency order

## Security

- Never commit API tokens
- Use environment variables for sensitive data
- Verify all downloads with checksums
- Keep dependencies up to date
