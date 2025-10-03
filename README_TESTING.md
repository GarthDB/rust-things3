# Testing the Rust Things3 MCP with Real Data

This document explains how to test the Rust Things3 MCP implementation using your actual Things 3 app data on macOS.

## Overview

We've created several test scripts that safely work with your real Things 3 database to validate the MCP server implementation. All scripts operate in read-only mode to ensure your data remains safe.

## Test Scripts

### 1. Python Test Script (`test_mcp_with_real_data.py`)

A comprehensive Python test runner with the following features:

- **Safe Testing**: Creates temporary copies of your database for testing
- **Comprehensive Coverage**: Tests health checks, basic operations, MCP server, and performance
- **Backup Support**: Can use backup databases instead of live data
- **Multiple Output Formats**: Regular terminal output or JSON for automation
- **Performance Benchmarking**: Optional performance tests

#### Usage

```bash
# Basic test run
python test_mcp_with_real_data.py

# Build CLI first, then test
python test_mcp_with_real_data.py --build

# Use backup database for testing
python test_mcp_with_real_data.py --backup-test

# Run with performance benchmarks
python test_mcp_with_real_data.py --performance

# Verbose output
python test_mcp_with_real_data.py --verbose

# JSON output for automation
python test_mcp_with_real_data.py --json-output
```

### 2. Interactive Bash Script (`test_mcp_interactive.sh`)

An interactive menu-driven test script perfect for manual testing and exploration:

- **Interactive Menu**: Easy-to-use menu interface
- **Step-by-Step Testing**: Run individual test components
- **Database Information**: View your database statistics
- **Sample Queries**: See example commands you can run
- **Finder Integration**: Open database location in Finder

#### Usage

```bash
# Run interactive test script
./test_mcp_interactive.sh
```

The script provides a menu with options:
1. Check prerequisites
2. Show database information  
3. Test basic CLI commands
4. Test MCP server startup
5. Run all tests
6. Build CLI
7. Open database location in Finder
8. Show sample queries

### 3. Rust Test Binary (`src/bin/test_mcp_real_data.rs`)

A native Rust test binary that integrates directly with the Things3 core library:

- **Native Integration**: Uses the same code as the CLI
- **Performance Testing**: Built-in benchmarking capabilities
- **Schema Validation**: Validates database schema compatibility
- **MCP Protocol Simulation**: Simulates actual MCP tool calls

#### Usage

```bash
# Build and run the test binary
cargo run --bin test_mcp_real_data

# With custom database path
cargo run --bin test_mcp_real_data -- --database-path /path/to/things.db

# With performance tests
cargo run --bin test_mcp_real_data -- --performance

# Verbose output
cargo run --bin test_mcp_real_data -- --verbose

# Dry run (check setup only)
cargo run --bin test_mcp_real_data -- --dry-run
```

## Database Location

Your Things 3 database is located at:
```
/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite
```

### Backup Databases

Things 3 automatically creates backups in:
```
/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Backups/
```

The test scripts can automatically use the latest backup for safer testing.

## Prerequisites

Before running the tests, ensure:

1. **Things 3 is installed** and has data
2. **CLI is built**: Run `cargo build --release` 
3. **Python 3** (for Python script)
4. **jq** (optional, for JSON processing in bash script)
5. **sqlite3** (optional, for database analysis)

## Safety Features

All test scripts include safety features:

- **Read-Only Mode**: Database is accessed in read-only mode
- **Temporary Copies**: Python script creates temporary database copies
- **Backup Support**: Option to use backup databases instead of live data
- **No Data Modification**: Tests only read data, never write
- **Error Handling**: Comprehensive error handling and cleanup

## What the Tests Validate

### 1. Database Connectivity
- Can connect to the Things 3 database
- Database schema is as expected
- Required tables exist

### 2. Basic Data Retrieval
- Inbox tasks can be retrieved
- Today's tasks can be retrieved  
- Projects can be retrieved
- Areas can be retrieved
- Data structures are valid

### 3. MCP Server Functionality
- MCP server can start
- MCP tools work correctly
- JSON serialization works
- Error handling works

### 4. Performance
- Query response times
- Memory usage
- Concurrent access handling

## Example Output

```bash
$ python test_mcp_with_real_data.py

[HEADER] Checking prerequisites...
[SUCCESS] ✓ Things3 database found
[SUCCESS] ✓ CLI binary found
[SUCCESS] ✓ Database accessible with 15 tables

[HEADER] Setting up test database...
[SUCCESS] ✓ Test database created: /tmp/things3_test_xyz/test_things.sqlite

[HEADER] Analyzing database content...
[INFO] TMTask: 1,247 records
[INFO] TMProject: 23 records  
[INFO] TMArea: 8 records

[HEADER] Testing basic operations...
[SUCCESS] ✓ inbox tasks: 42 items
[SUCCESS] ✓ limited inbox tasks: 5 items
[SUCCESS] ✓ today's tasks: 7 items
[SUCCESS] ✓ projects: 23 items
[SUCCESS] ✓ areas: 8 items

[HEADER] Testing MCP server...
[SUCCESS] ✓ MCP server appears to start correctly

[SUCCESS] ✓ Test database cleaned up

=== Things3 MCP Test Report ===
Timestamp: 2024-01-15 14:30:22
Database: /tmp/things3_test_xyz/test_things.sqlite
Results Summary:
  ✓ database_analysis
  ✓ health_check
  ✓ basic_operations
  ✓ mcp_server
```

## Troubleshooting

### Database Not Found
```bash
[ERROR] Things3 database not found at: /Users/garthdb/Library/...
```
**Solution**: Ensure Things 3 is installed and has been opened at least once.

### CLI Binary Not Found
```bash
[WARNING] CLI binary not found at: target/release/things3-cli
```
**Solution**: Run `cargo build --release` or use the `--build` flag.

### Permission Denied
```bash
[ERROR] Cannot access database: Permission denied
```
**Solution**: Ensure Things 3 is not running, or use a backup database.

### Build Errors
If the CLI fails to build, check:
- Rust toolchain is up to date: `rustup update`
- All dependencies are available
- System has enough disk space

## Manual Testing

You can also test manually using the CLI directly:

```bash
# Set the database path
export THINGS_DB_PATH="/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"

# Test basic commands
./target/release/things3 health
./target/release/things3 inbox --limit 5
./target/release/things3 today
./target/release/things3 projects
./target/release/things3 areas

# Start MCP server (for AI/LLM integration)
./target/release/things3 mcp
```

## Integration with AI Tools

Once testing is complete, you can integrate the MCP server with AI tools like Cursor:

```json
{
  "mcpServers": {
    "things3": {
      "command": "things3",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"
      }
    }
  }
}
```

## Next Steps

After successful testing:

1. **Production Use**: Start using the MCP server with your AI tools
2. **Monitoring**: Use the health check endpoints for monitoring
3. **Backup Strategy**: Ensure regular backups of your Things 3 data
4. **Performance Tuning**: Adjust cache settings based on test results
5. **Feature Requests**: Submit issues/PRs for additional functionality

## Contributing

If you find issues or want to improve the tests:

1. Report bugs with test output and system information
2. Submit PRs with additional test cases
3. Suggest improvements to safety features
4. Help with documentation
