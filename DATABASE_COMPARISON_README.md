# Things 3 Database Cross-Machine Comparison

This directory contains scripts to analyze and compare Things 3 databases between different Mac architectures (Apple Silicon vs Intel).

## Problem

The `things3` CLI works differently on Apple Silicon vs Intel Macs due to:
- Different database locations
- Potential schema differences
- Different data formats or endianness
- Architecture-specific Things 3 versions

## Solution

Two-step analysis process to identify and resolve compatibility issues.

## Step 1: Collect Intel Mac Data

**Run this on your Intel Mac:**

```bash
./diagnose_things3_database.sh
```

This will create an `intel_mac_analysis/` directory with comprehensive database analysis including:
- System information and architecture details
- Container discovery and database locations
- Complete database schema dumps
- Data format analysis
- Critical column compatibility checks
- Binary data samples for endianness testing
- Machine fingerprint for comparison

### Output Files

The script generates:
- `machine_fingerprint.json` - Machine identification data
- `schema_*.sql` - Complete database schema
- `critical_columns_*.txt` - Column compatibility analysis
- `binary_analysis_*.txt` - Binary data format analysis
- `container_*_analysis.txt` - Container discovery results
- `INTEL_MAC_SUMMARY.md` - Summary of findings

## Step 2: Transfer and Compare

**Copy the analysis to your Apple Silicon Mac:**

```bash
# From Intel Mac
scp -r intel_mac_analysis/ user@apple-silicon-mac:/Users/garthdb/Projects/rust-things3/

# On Apple Silicon Mac
./compare_machine_databases.sh
```

This will:
1. Generate equivalent analysis for the Apple Silicon Mac
2. Compare all aspects between the two machines
3. Create detailed comparison reports
4. Generate recommendations for fixing compatibility issues

### Comparison Output

The comparison script creates a `machine_comparison/` directory with:
- `fingerprint_comparison.md` - Machine and path differences
- `schema_comparison.md` - Database schema differences
- `critical_columns_comparison.md` - Column compatibility matrix
- `binary_data_comparison.md` - Binary format analysis
- `FINAL_COMPARISON_REPORT.md` - Executive summary and recommendations

## Expected Findings

Based on the current CLI issues, we expect to find:

1. **Database Path Differences**
   - Different container identifiers
   - Different ThingsData directory names
   - Migration-related path changes

2. **Schema Differences**
   - Missing `parent` column (Apple Silicon uses `heading`)
   - Different column names or types
   - Architecture-specific schema evolution

3. **Binary Format Differences**
   - Endianness differences in BLOB data
   - Timestamp precision variations
   - SQLite version differences

## Using the Results

Once you have the comparison results:

1. **Review the Final Report** - Start with `FINAL_COMPARISON_REPORT.md`
2. **Update CLI Code** - Modify database models and queries based on findings
3. **Implement Dynamic Detection** - Add runtime architecture/schema detection
4. **Test Compatibility** - Verify CLI works on both architectures

## Files in This Analysis

- `diagnose_things3_database.sh` - Intel Mac data collection script
- `compare_machine_databases.sh` - Cross-machine comparison script
- `DATABASE_COMPARISON_README.md` - This documentation

## Troubleshooting

### Intel Mac Script Issues
- **No containers found**: Check if Things 3 is installed and has been run
- **Database access denied**: Ensure Things 3 is not running during analysis
- **SQLite not found**: Install SQLite3 command-line tools

### Comparison Script Issues
- **Missing intel_mac_analysis**: Ensure you've copied the directory from Intel Mac
- **Apple Silicon database not found**: Update the hardcoded path in the comparison script
- **Permission errors**: Ensure both scripts are executable (`chmod +x *.sh`)

## Security Note

These scripts read database files but do not modify them. They create analysis files that may contain task titles and other personal data. Review generated files before sharing.


