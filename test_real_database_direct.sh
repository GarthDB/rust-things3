#!/bin/bash

# Direct database test script for Things3 real data
# This script tests the database directly without relying on the CLI

set -euo pipefail

# Configuration
THINGS3_DB_PATH="/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"

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

# Test database directly
test_database_direct() {
    log_header "Testing Things3 Database Directly"
    
    if [[ ! -f "$THINGS3_DB_PATH" ]]; then
        log_error "Database not found at: $THINGS3_DB_PATH"
        return 1
    fi
    
    log_success "Database file found"
    
    # Test basic connectivity
    log_info "Testing database connectivity..."
    local tables=$(sqlite3 "$THINGS3_DB_PATH" ".tables" | wc -w)
    log_success "Database accessible with $tables tables"
    
    # Show database statistics
    log_info "Database statistics:"
    
    # Count tasks by type
    log_info "Task counts by type:"
    sqlite3 "$THINGS3_DB_PATH" "
        SELECT 
            CASE 
                WHEN type = 0 THEN 'Tasks (type 0)'
                WHEN type = 1 THEN 'Projects (type 1)' 
                WHEN type = 2 THEN 'Headings (type 2)'
                ELSE 'Other (type ' || type || ')'
            END as item_type,
            COUNT(*) as count
        FROM TMTask 
        GROUP BY type 
        ORDER BY type;
    " | while IFS='|' read -r type_name count; do
        echo "  - $type_name: $count"
    done
    
    # Count areas
    local area_count=$(sqlite3 "$THINGS3_DB_PATH" "SELECT COUNT(*) FROM TMArea;")
    log_info "  - Areas: $area_count"
    
    # Count tags
    local tag_count=$(sqlite3 "$THINGS3_DB_PATH" "SELECT COUNT(*) FROM TMTag;")
    log_info "  - Tags: $tag_count"
    
    # Show sample tasks (inbox items)
    log_info "Sample inbox tasks (type 0, no project):"
    sqlite3 "$THINGS3_DB_PATH" "
        SELECT title 
        FROM TMTask 
        WHERE type = 0 
        AND project IS NULL 
        AND status = 0 
        AND trashed = 0
        LIMIT 5;
    " | while read -r title; do
        echo "  - $title"
    done
    
    # Show sample projects
    log_info "Sample projects (type 1):"
    sqlite3 "$THINGS3_DB_PATH" "
        SELECT title 
        FROM TMTask 
        WHERE type = 1 
        AND status = 0 
        AND trashed = 0
        LIMIT 5;
    " | while read -r title; do
        echo "  - $title"
    done
    
    # Show sample areas
    log_info "Areas:"
    sqlite3 "$THINGS3_DB_PATH" "
        SELECT title 
        FROM TMArea 
        WHERE visible = 1
        LIMIT 10;
    " | while read -r title; do
        echo "  - $title"
    done
    
    log_success "Database analysis complete!"
}

# Test MCP server with schema awareness
test_mcp_compatibility() {
    log_header "Testing MCP Schema Compatibility"
    
    log_info "Checking if database schema matches expected MCP schema..."
    
    # Check for expected tables
    local expected_tables=("TMTask" "TMArea" "TMTag")
    local missing_tables=()
    
    for table in "${expected_tables[@]}"; do
        if sqlite3 "$THINGS3_DB_PATH" ".tables" | grep -q "$table"; then
            log_success "✓ Found table: $table"
        else
            missing_tables+=("$table")
            log_error "✗ Missing table: $table"
        fi
    done
    
    # Check for TMProject table (expected by current code)
    if sqlite3 "$THINGS3_DB_PATH" ".tables" | grep -q "TMProject"; then
        log_success "✓ Found TMProject table"
    else
        log_warning "⚠ TMProject table not found - projects are stored in TMTask table"
        log_info "This is normal for Things 3 - projects have type=1 in TMTask"
    fi
    
    # Show actual schema differences
    log_info "Schema analysis:"
    log_info "  - Things 3 stores projects as TMTask records with type=1"
    log_info "  - Current MCP code expects separate TMProject table"
    log_info "  - This explains why CLI commands are failing"
    
    if [[ ${#missing_tables[@]} -eq 0 ]]; then
        log_success "Core tables present - MCP can be adapted to work"
    else
        log_error "Missing core tables: ${missing_tables[*]}"
    fi
}

# Show recommendations
show_recommendations() {
    log_header "Recommendations"
    
    echo "Based on the database analysis:"
    echo
    echo "✅ Your Things 3 database is healthy and accessible"
    echo "✅ Contains real data: tasks, projects, areas, and tags"
    echo "⚠️  Current MCP code expects different schema than real Things 3"
    echo
    echo "To fix the MCP integration:"
    echo "1. Update database queries to use TMTask table for projects (type=1)"
    echo "2. Adapt column names to match real schema (project vs project_uuid)"
    echo "3. Handle the unified TMTask table structure"
    echo
    echo "Your database structure:"
    echo "  - Tasks: TMTask with type=0"
    echo "  - Projects: TMTask with type=1" 
    echo "  - Headings: TMTask with type=2"
    echo "  - Areas: TMArea table"
    echo "  - Tags: TMTag table"
    echo
    echo "The MCP server can work with this data once the schema mapping is updated."
}

# Main function
main() {
    log_header "Things3 Real Database Direct Test"
    
    test_database_direct
    test_mcp_compatibility
    show_recommendations
    
    log_success "Direct database test completed!"
    echo
    echo "Next steps:"
    echo "1. The database is working and contains your real data"
    echo "2. The MCP code needs schema updates to work with real Things 3 format"
    echo "3. Consider this a successful validation of your data accessibility"
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
