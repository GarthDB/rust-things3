#!/usr/bin/env python3
"""
Test script for Rust Things3 MCP using real app data

This script tests the Things3 MCP server implementation using your actual Things 3 database.
It safely operates in read-only mode to avoid modifying your real data.

Usage:
    python test_mcp_with_real_data.py [options]

Options:
    --verbose       Enable verbose output
    --build         Build the CLI before testing
    --backup-test   Test using a backup database instead of live data
    --json-output   Output results in JSON format
    --performance   Run performance benchmarks
"""

import os
import sys
import json
import time
import subprocess
import asyncio
import tempfile
import shutil
import argparse
from pathlib import Path
from typing import Dict, List, Any, Optional
import sqlite3

# Configuration
THINGS3_DB_PATH = "/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"
PROJECT_ROOT = Path(__file__).parent
CLI_BINARY = PROJECT_ROOT / "target/release/things3"
BACKUP_DIR = "/Users/garthdb/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Backups"

class Colors:
    """ANSI color codes for terminal output"""
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

class Things3TestRunner:
    """Test runner for Things3 MCP with real data"""
    
    def __init__(self, verbose: bool = False, json_output: bool = False):
        self.verbose = verbose
        self.json_output = json_output
        self.results = {}
        self.test_db_path = None
        
    def log(self, message: str, level: str = "INFO") -> None:
        """Log a message with appropriate formatting"""
        if self.json_output:
            return
            
        color = {
            "INFO": Colors.OKBLUE,
            "SUCCESS": Colors.OKGREEN,
            "WARNING": Colors.WARNING,
            "ERROR": Colors.FAIL,
            "HEADER": Colors.HEADER
        }.get(level, "")
        
        if self.verbose or level in ["SUCCESS", "ERROR", "HEADER"]:
            print(f"{color}[{level}]{Colors.ENDC} {message}")
    
    def check_prerequisites(self) -> bool:
        """Check if all prerequisites are met"""
        self.log("Checking prerequisites...", "HEADER")
        
        # Check if Things3 database exists
        if not Path(THINGS3_DB_PATH).exists():
            self.log(f"Things3 database not found at: {THINGS3_DB_PATH}", "ERROR")
            return False
        self.log("✓ Things3 database found", "SUCCESS")
        
        # Check if CLI binary exists
        if not CLI_BINARY.exists():
            self.log(f"CLI binary not found at: {CLI_BINARY}", "WARNING")
            self.log("Run with --build to build the CLI first", "INFO")
            return False
        self.log("✓ CLI binary found", "SUCCESS")
        
        # Check if database is accessible
        try:
            conn = sqlite3.connect(THINGS3_DB_PATH)
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
            table_count = cursor.fetchone()[0]
            conn.close()
            self.log(f"✓ Database accessible with {table_count} tables", "SUCCESS")
        except Exception as e:
            self.log(f"Cannot access database: {e}", "ERROR")
            return False
            
        return True
    
    def build_cli(self) -> bool:
        """Build the CLI binary"""
        self.log("Building CLI binary...", "HEADER")
        
        try:
            result = subprocess.run(
                ["cargo", "build", "--release"],
                cwd=PROJECT_ROOT,
                capture_output=True,
                text=True,
                timeout=300  # 5 minutes timeout
            )
            
            if result.returncode == 0:
                self.log("✓ CLI built successfully", "SUCCESS")
                return True
            else:
                self.log(f"Build failed: {result.stderr}", "ERROR")
                return False
                
        except subprocess.TimeoutExpired:
            self.log("Build timed out after 5 minutes", "ERROR")
            return False
        except Exception as e:
            self.log(f"Build error: {e}", "ERROR")
            return False
    
    def get_latest_backup(self) -> Optional[str]:
        """Get the latest backup database"""
        backup_path = Path(BACKUP_DIR)
        if not backup_path.exists():
            return None
            
        backups = list(backup_path.glob("*/main.sqlite"))
        if not backups:
            return None
            
        # Sort by modification time and get the latest
        latest_backup = max(backups, key=lambda p: p.stat().st_mtime)
        return str(latest_backup)
    
    def setup_test_database(self, use_backup: bool = False) -> bool:
        """Setup a test database (copy of real data)"""
        self.log("Setting up test database...", "HEADER")
        
        source_db = THINGS3_DB_PATH
        if use_backup:
            backup_db = self.get_latest_backup()
            if backup_db:
                source_db = backup_db
                self.log(f"Using backup database: {backup_db}", "INFO")
            else:
                self.log("No backup found, using live database", "WARNING")
        
        try:
            # Create temporary database
            temp_dir = tempfile.mkdtemp(prefix="things3_test_")
            self.test_db_path = os.path.join(temp_dir, "test_things.sqlite")
            
            # Copy database
            shutil.copy2(source_db, self.test_db_path)
            
            # Make it read-only for safety
            os.chmod(self.test_db_path, 0o444)
            
            self.log(f"✓ Test database created: {self.test_db_path}", "SUCCESS")
            return True
            
        except Exception as e:
            self.log(f"Failed to setup test database: {e}", "ERROR")
            return False
    
    def cleanup_test_database(self) -> None:
        """Clean up test database"""
        if self.test_db_path and os.path.exists(self.test_db_path):
            try:
                # Remove read-only flag
                os.chmod(self.test_db_path, 0o644)
                os.remove(self.test_db_path)
                
                # Remove temp directory
                temp_dir = os.path.dirname(self.test_db_path)
                os.rmdir(temp_dir)
                
                self.log("✓ Test database cleaned up", "SUCCESS")
            except Exception as e:
                self.log(f"Cleanup warning: {e}", "WARNING")
    
    def run_cli_command(self, args: List[str], timeout: int = 30) -> Dict[str, Any]:
        """Run a CLI command and return the result"""
        cmd = [str(CLI_BINARY)] + args
        
        # Set environment to use test database
        env = os.environ.copy()
        env["THINGS_DB_PATH"] = self.test_db_path
        env["RUST_LOG"] = "info" if self.verbose else "warn"
        
        try:
            start_time = time.time()
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                env=env
            )
            duration = time.time() - start_time
            
            return {
                "success": result.returncode == 0,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "duration": duration,
                "command": " ".join(args)
            }
            
        except subprocess.TimeoutExpired:
            return {
                "success": False,
                "stdout": "",
                "stderr": f"Command timed out after {timeout}s",
                "duration": timeout,
                "command": " ".join(args)
            }
        except Exception as e:
            return {
                "success": False,
                "stdout": "",
                "stderr": str(e),
                "duration": 0,
                "command": " ".join(args)
            }
    
    def test_health_check(self) -> Dict[str, Any]:
        """Test basic health check"""
        self.log("Testing health check...", "INFO")
        result = self.run_cli_command(["health"])
        
        if result["success"]:
            self.log("✓ Health check passed", "SUCCESS")
        else:
            self.log(f"✗ Health check failed: {result['stderr']}", "ERROR")
        
        return result
    
    def test_basic_operations(self) -> Dict[str, Any]:
        """Test basic CLI operations with real data"""
        self.log("Testing basic operations...", "HEADER")
        
        operations = [
            (["inbox"], "inbox tasks"),
            (["inbox", "--limit", "5"], "limited inbox tasks"),
            (["today"], "today's tasks"),
            (["projects"], "projects"),
            (["areas"], "areas"),
        ]
        
        results = {}
        for args, description in operations:
            self.log(f"Testing {description}...", "INFO")
            result = self.run_cli_command(args)
            results[description] = result
            
            if result["success"]:
                # Try to parse as JSON to validate output
                try:
                    data = json.loads(result["stdout"])
                    count = len(data) if isinstance(data, list) else 1
                    self.log(f"✓ {description}: {count} items", "SUCCESS")
                except json.JSONDecodeError:
                    self.log(f"✓ {description}: non-JSON output", "SUCCESS")
            else:
                self.log(f"✗ {description} failed: {result['stderr']}", "ERROR")
        
        return results
    
    def analyze_database_content(self) -> Dict[str, Any]:
        """Analyze the real database content"""
        self.log("Analyzing database content...", "HEADER")
        
        try:
            conn = sqlite3.connect(self.test_db_path)
            cursor = conn.cursor()
            
            # Get table information
            cursor.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            tables = [row[0] for row in cursor.fetchall()]
            
            analysis = {"tables": tables, "counts": {}}
            
            # Get counts for main tables
            main_tables = ["TMTask", "TMProject", "TMArea", "TMTag"]
            for table in main_tables:
                if table in tables:
                    cursor.execute(f"SELECT COUNT(*) FROM {table}")
                    count = cursor.fetchone()[0]
                    analysis["counts"][table] = count
                    self.log(f"{table}: {count} records", "INFO")
            
            conn.close()
            return analysis
            
        except Exception as e:
            self.log(f"Database analysis failed: {e}", "ERROR")
            return {"error": str(e)}
    
    def test_mcp_server(self) -> Dict[str, Any]:
        """Test MCP server functionality"""
        self.log("Testing MCP server...", "HEADER")
        
        # Start MCP server in background
        env = os.environ.copy()
        env["THINGS_DB_PATH"] = self.test_db_path
        env["RUST_LOG"] = "info" if self.verbose else "warn"
        
        try:
            # For now, just test that MCP command starts without immediate error
            # In a real implementation, you'd want to test the actual MCP protocol
            result = self.run_cli_command(["mcp"], timeout=5)
            
            # Since MCP server is meant to run continuously, we expect it to timeout
            # or run successfully for the timeout period
            if "server started" in result["stderr"].lower() or result["duration"] >= 4:
                self.log("✓ MCP server appears to start correctly", "SUCCESS")
                return {"success": True, "note": "MCP server startup test"}
            else:
                self.log(f"✗ MCP server test failed: {result['stderr']}", "ERROR")
                return result
                
        except Exception as e:
            self.log(f"MCP server test error: {e}", "ERROR")
            return {"success": False, "error": str(e)}
    
    def run_performance_tests(self) -> Dict[str, Any]:
        """Run performance benchmarks"""
        self.log("Running performance tests...", "HEADER")
        
        performance_tests = [
            (["inbox"], "inbox_query"),
            (["projects"], "projects_query"),
            (["areas"], "areas_query"),
            (["health"], "health_check"),
        ]
        
        results = {}
        for args, test_name in performance_tests:
            self.log(f"Benchmarking {test_name}...", "INFO")
            
            # Run test multiple times
            durations = []
            for i in range(3):
                result = self.run_cli_command(args)
                if result["success"]:
                    durations.append(result["duration"])
                else:
                    self.log(f"Performance test failed on run {i+1}", "WARNING")
                    break
            
            if durations:
                avg_duration = sum(durations) / len(durations)
                min_duration = min(durations)
                max_duration = max(durations)
                
                results[test_name] = {
                    "avg_duration": avg_duration,
                    "min_duration": min_duration,
                    "max_duration": max_duration,
                    "runs": len(durations)
                }
                
                self.log(f"✓ {test_name}: avg {avg_duration:.3f}s (min: {min_duration:.3f}s, max: {max_duration:.3f}s)", "SUCCESS")
            else:
                results[test_name] = {"error": "All runs failed"}
                self.log(f"✗ {test_name}: all runs failed", "ERROR")
        
        return results
    
    def generate_test_report(self) -> None:
        """Generate a comprehensive test report"""
        self.log("Generating test report...", "HEADER")
        
        report = {
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "database_path": self.test_db_path,
            "results": self.results
        }
        
        if self.json_output:
            print(json.dumps(report, indent=2))
        else:
            print(f"\n{Colors.HEADER}=== Things3 MCP Test Report ==={Colors.ENDC}")
            print(f"Timestamp: {report['timestamp']}")
            print(f"Database: {report['database_path']}")
            print(f"\nResults Summary:")
            
            for test_name, result in self.results.items():
                if isinstance(result, dict) and result.get("success", False):
                    print(f"  ✓ {test_name}")
                else:
                    print(f"  ✗ {test_name}")
    
    def run_all_tests(self, use_backup: bool = False, run_performance: bool = False) -> bool:
        """Run all tests"""
        self.log("Starting Things3 MCP tests with real data", "HEADER")
        
        try:
            # Setup test database
            if not self.setup_test_database(use_backup):
                return False
            
            # Analyze database
            self.results["database_analysis"] = self.analyze_database_content()
            
            # Run health check
            self.results["health_check"] = self.test_health_check()
            
            # Run basic operations
            self.results["basic_operations"] = self.test_basic_operations()
            
            # Test MCP server
            self.results["mcp_server"] = self.test_mcp_server()
            
            # Run performance tests if requested
            if run_performance:
                self.results["performance"] = self.run_performance_tests()
            
            # Generate report
            self.generate_test_report()
            
            return True
            
        finally:
            self.cleanup_test_database()

def main():
    parser = argparse.ArgumentParser(description="Test Rust Things3 MCP with real data")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--build", action="store_true", help="Build CLI before testing")
    parser.add_argument("--backup-test", action="store_true", help="Use backup database")
    parser.add_argument("--json-output", action="store_true", help="Output in JSON format")
    parser.add_argument("--performance", action="store_true", help="Run performance tests")
    
    args = parser.parse_args()
    
    runner = Things3TestRunner(verbose=args.verbose, json_output=args.json_output)
    
    # Check prerequisites
    if not runner.check_prerequisites():
        if args.build:
            if not runner.build_cli():
                sys.exit(1)
            # Re-check after build
            if not runner.check_prerequisites():
                sys.exit(1)
        else:
            sys.exit(1)
    
    # Run tests
    success = runner.run_all_tests(
        use_backup=args.backup_test,
        run_performance=args.performance
    )
    
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
