#!/bin/bash
# Coverage script for Rust Things monorepo

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo -e "${BLUE}🔍 Running code coverage analysis for Rust Things monorepo${NC}"
echo

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo -e "${RED}❌ cargo-llvm-cov is not installed${NC}"
    echo "Install it with: cargo install cargo-llvm-cov"
    exit 1
fi

# Check if LLVM tools are installed
if ! rustup component list --installed | grep -q "llvm-tools-preview"; then
    echo -e "${YELLOW}⚠️  LLVM tools not installed, installing...${NC}"
    rustup component add llvm-tools-preview
fi

echo -e "${BLUE}📊 Generating coverage reports...${NC}"

# Clean previous coverage data
echo -e "${YELLOW}🧹 Cleaning previous coverage data...${NC}"
cargo llvm-cov clean

# Generate coverage reports
echo -e "${BLUE}🔍 Running tests with coverage...${NC}"

# Generate LCOV report
echo -e "${YELLOW}📊 Generating LCOV report...${NC}"
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

# Generate HTML report
echo -e "${YELLOW}📊 Generating HTML report...${NC}"
cargo llvm-cov --workspace --all-features --html --output-dir target/llvm-cov/html

# Generate JSON report
echo -e "${YELLOW}📊 Generating JSON report...${NC}"
cargo llvm-cov --workspace --all-features --json --output-path coverage.json

# Generate Cobertura report
echo -e "${YELLOW}📊 Generating Cobertura report...${NC}"
cargo llvm-cov --workspace --all-features --cobertura --output-path cobertura.xml

# Generate text report
echo -e "${YELLOW}📊 Generating text report...${NC}"
cargo llvm-cov --workspace --all-features --text --output-path coverage.txt

echo
echo -e "${GREEN}✅ Coverage analysis complete!${NC}"
echo

# Display coverage summary
echo -e "${BLUE}📈 Coverage Summary:${NC}"
if [ -f "coverage.txt" ]; then
    echo
    cat coverage.txt
    echo
fi

# Display file locations
echo -e "${BLUE}📁 Generated Reports:${NC}"
echo "  • HTML Report: target/llvm-cov/html/index.html"
echo "  • LCOV File: lcov.info"
echo "  • JSON File: coverage.json"
echo "  • Cobertura File: cobertura.xml"
echo "  • Text File: coverage.txt"
echo

# Open HTML report if on macOS
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo -e "${BLUE}🌐 Opening HTML report in browser...${NC}"
    open target/llvm-cov/html/index.html
fi

echo -e "${GREEN}🎉 Coverage analysis complete!${NC}"
