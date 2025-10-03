#!/bin/bash

# Interactive test script for Things3 MCP with real data
# This script provides an interactive way to test the MCP server

set -euo pipefail

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
THINGS3_DB_PATH="/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"
CLI_BINARY="$PROJECT_ROOT/target/release/things3"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_header() {
    echo -e "\n${PURPLE}=== $1 ===${NC}"
}

# Check prerequisites
check_prerequisites() {
    log_header "Checking Prerequisites"
    
    # Check if Things3 database exists
    if [[ ! -f "$THINGS3_DB_PATH" ]]; then
        log_error "Things3 database not found at: $THINGS3_DB_PATH"
        return 1
    fi
    log_success "Things3 database found"
    
    # Check if CLI binary exists or offer to build
    if [[ ! -f "$CLI_BINARY" ]]; then
        log_warning "CLI binary not found at: $CLI_BINARY"
        echo -n "Would you like to build it now? (y/N): "
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            build_cli
        else
            log_error "CLI binary is required. Run 'cargo build --release' first."
            return 1
        fi
    fi
    log_success "CLI binary found"
    
    return 0
}

# Build CLI
build_cli() {
    log_header "Building CLI"
    cd "$PROJECT_ROOT"
    
    log_info "Building release binary..."
    if cargo build --release; then
        log_success "CLI built successfully"
    else
        log_error "Build failed"
        return 1
    fi
}

# Show database info
show_database_info() {
    log_header "Database Information"
    
    # Use sqlite3 to get basic info
    if command -v sqlite3 &> /dev/null; then
        log_info "Database file: $THINGS3_DB_PATH"
        
        # Get file size
        local size=$(stat -f%z "$THINGS3_DB_PATH" 2>/dev/null || echo "Unknown")
        log_info "Database size: $size bytes"
        
        # Get table count
        local table_count=$(sqlite3 "$THINGS3_DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table';" 2>/dev/null || echo "0")
        log_info "Number of tables: $table_count"
        
        # Get basic record counts
        log_info "Record counts:"
        local tasks=$(sqlite3 "$THINGS3_DB_PATH" "SELECT COUNT(*) FROM TMTask;" 2>/dev/null || echo "N/A")
        local projects=$(sqlite3 "$THINGS3_DB_PATH" "SELECT COUNT(*) FROM TMProject;" 2>/dev/null || echo "N/A")
        local areas=$(sqlite3 "$THINGS3_DB_PATH" "SELECT COUNT(*) FROM TMArea;" 2>/dev/null || echo "N/A")
        
        echo "  - Tasks: $tasks"
        echo "  - Projects: $projects"
        echo "  - Areas: $areas"
    else
        log_warning "sqlite3 not available, skipping database analysis"
    fi
}

# Test basic CLI commands
test_basic_commands() {
    log_header "Testing Basic Commands"
    
    local commands=(
        "health:Health check"
        "inbox --limit 5:Inbox (limited)"
        "today --limit 3:Today's tasks (limited)"
        "projects --limit 5:Projects (limited)"
        "areas:Areas"
    )
    
    export THINGS_DB_PATH="$THINGS3_DB_PATH"
    export RUST_LOG="info"
    
    for cmd_desc in "${commands[@]}"; do
        IFS=':' read -r cmd desc <<< "$cmd_desc"
        
        log_info "Testing: $desc"
        echo -n "  Running '$CLI_BINARY $cmd'... "
        
        if timeout 30s "$CLI_BINARY" $cmd > /tmp/things3_test_output.json 2>&1; then
            echo -e "${GREEN}✓${NC}"
            
            # Try to count results if it's JSON
            if jq empty /tmp/things3_test_output.json 2>/dev/null; then
                local count=$(jq '. | length' /tmp/things3_test_output.json 2>/dev/null || echo "N/A")
                echo "    Results: $count items"
            else
                echo "    Output: $(wc -l < /tmp/things3_test_output.json) lines"
            fi
        else
            echo -e "${RED}✗${NC}"
            echo "    Error: $(head -1 /tmp/things3_test_output.json)"
        fi
    done
    
    rm -f /tmp/things3_test_output.json
}

# Test MCP server (basic startup test)
test_mcp_server() {
    log_header "Testing MCP Server"
    
    export THINGS_DB_PATH="$THINGS3_DB_PATH"
    export RUST_LOG="info"
    
    log_info "Testing MCP server startup..."
    
    # Start MCP server in background with timeout
    if timeout 10s "$CLI_BINARY" mcp > /tmp/mcp_test.log 2>&1 &
    then
        local mcp_pid=$!
        
        # Wait a moment for startup
        sleep 3
        
        # Check if process is still running
        if kill -0 $mcp_pid 2>/dev/null; then
            log_success "MCP server started successfully"
            
            # Kill the server
            kill $mcp_pid 2>/dev/null || true
            wait $mcp_pid 2>/dev/null || true
        else
            log_error "MCP server failed to start or crashed"
            echo "Log output:"
            cat /tmp/mcp_test.log
        fi
    else
        log_error "Failed to start MCP server"
    fi
    
    rm -f /tmp/mcp_test.log
}

# Interactive menu
show_menu() {
    echo
    echo -e "${CYAN}=== Things3 MCP Test Menu ===${NC}"
    echo "1) Check prerequisites"
    echo "2) Show database information"
    echo "3) Test basic CLI commands"
    echo "4) Test MCP server startup"
    echo "5) Run all tests"
    echo "6) Build CLI"
    echo "7) Open database location in Finder"
    echo "8) Show sample queries"
    echo "q) Quit"
    echo
}

# Show sample queries
show_sample_queries() {
    log_header "Sample Queries"
    
    cat << 'EOF'
Here are some sample commands you can run manually:

# Basic queries (set THINGS_DB_PATH first)
export THINGS_DB_PATH="/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"

# Health check
./target/release/things3 health

# Get inbox tasks
./target/release/things3 inbox

# Get today's tasks
./target/release/things3 today

# Get projects in a specific area (replace UUID)
./target/release/things3 projects --area YOUR_AREA_UUID

# Start MCP server (for use with AI tools)
./target/release/things3 mcp

# Direct SQLite queries (if you have sqlite3)
sqlite3 "$THINGS_DB_PATH" "SELECT title FROM TMTask WHERE status = 0 LIMIT 5;"

# Backup database analysis
ls -la "/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Backups/"
EOF
}

# Main interactive loop
main() {
    log_header "Things3 MCP Interactive Test Script"
    
    while true; do
        show_menu
        echo -n "Choose an option: "
        read -r choice
        
        case $choice in
            1)
                check_prerequisites
                ;;
            2)
                show_database_info
                ;;
            3)
                if check_prerequisites; then
                    test_basic_commands
                fi
                ;;
            4)
                if check_prerequisites; then
                    test_mcp_server
                fi
                ;;
            5)
                if check_prerequisites; then
                    show_database_info
                    test_basic_commands
                    test_mcp_server
                    log_success "All tests completed!"
                fi
                ;;
            6)
                build_cli
                ;;
            7)
                log_info "Opening database location in Finder..."
                open "/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/"
                ;;
            8)
                show_sample_queries
                ;;
            q|Q)
                log_info "Goodbye!"
                exit 0
                ;;
            *)
                log_warning "Invalid option. Please try again."
                ;;
        esac
        
        echo
        echo -n "Press Enter to continue..."
        read -r
    done
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
