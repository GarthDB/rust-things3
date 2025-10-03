# Quick Start: Testing Rust Things3 MCP with Real Data

This guide gets you up and running quickly with testing the Rust Things3 MCP implementation using your actual Things 3 data.

## 🚀 Quick Test (30 seconds)

```bash
# 1. Build and run the interactive test
./test_mcp_interactive.sh

# 2. Choose option 5 to "Run all tests"
```

## 📋 What You Get

Three different test approaches for comprehensive validation:

### 1. 🖥️ Interactive Test (Recommended for first-time users)
```bash
./test_mcp_interactive.sh
```

**Features:**
- Menu-driven interface
- Step-by-step testing
- Real-time database stats
- Safe testing mode

### 2. 🐍 Python Comprehensive Test
```bash
# Basic test
python test_mcp_with_real_data.py

# Build first, then test with performance benchmarks
python test_mcp_with_real_data.py --build --performance --verbose
```

**Features:**
- Automated test suite
- Performance benchmarking
- JSON output for CI/CD
- Backup database support

### 3. 🦀 Native Rust Test
```bash
# Build and run (after building the CLI)
cargo run --bin test-mcp-real-data -- --verbose --performance
```

**Features:**
- Native Rust performance
- Direct API testing
- Detailed validation
- Schema verification

## 🔧 Prerequisites

1. **Things 3 installed** with some data
2. **Rust toolchain**: `rustup update`
3. **Python 3** (for Python script)

## 📊 Your Database

- **Location**: `/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite`
- **Backups**: Automatically available in the Backups folder
- **Safety**: All tests operate in read-only mode

## ⚡ Common Commands

```bash
# Quick health check
export THINGS_DB_PATH="/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"
cargo build --release
./target/release/things3 health

# Start MCP server for AI integration
./target/release/things3 mcp

# Get your actual data
./target/release/things3 inbox --limit 5
./target/release/things3 projects
./target/release/things3 areas
```

## 🎯 Expected Results

✅ **Database connectivity**  
✅ **Basic data retrieval** (inbox, projects, areas, today's tasks)  
✅ **MCP server startup**  
✅ **JSON serialization**  
✅ **Performance benchmarks** (if requested)  

## 🐛 Troubleshooting

| Issue | Solution |
|-------|----------|
| Database not found | Ensure Things 3 is installed and opened once |
| CLI not found | Run `cargo build --release` first |
| Permission denied | Close Things 3 app or use backup database |
| Build errors | Check `rustup update` and dependencies |

## 🔄 Next Steps

After successful testing:

1. **Use with AI Tools**: Configure Cursor/VS Code with the MCP server
2. **Monitor Performance**: Use health check endpoints
3. **Backup Strategy**: Regular Things 3 backups recommended
4. **Report Issues**: Submit bugs with test output

## 📖 Full Documentation

- `README_TESTING.md` - Complete testing guide
- `configs/editors/` - AI tool configurations
- `docs/CLI.md` - Full CLI documentation

---

**Ready to test?** Run `./test_mcp_interactive.sh` and choose option 5! 🚀
