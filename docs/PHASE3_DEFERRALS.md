# Phase 3 Deferrals and Future Work

This document explains items that were intentionally deferred from Phase 3 (Performance & Reliability) to keep the scope manageable and deliver value quickly.

## Deferred Items

### 1. MCP Server Benchmarks

**Status**: Deferred to post-Phase 3  
**Reason**: MCP server benchmarks require more complex test infrastructure  
**Complexity**:
- Need to mock stdin/stdout I/O for realistic testing
- JSON-RPC message handling adds overhead to measurements
- Requires careful isolation from database benchmarks
- Would significantly increase CI runtime

**Plan**: 
- Create separate MCP-specific benchmark suite
- Use `MockIo` for controlled I/O testing
- Measure end-to-end tool call latency
- Track in follow-up issue

**Related Files**:
- Would add: `apps/things3-cli/benches/mcp_benchmarks.rs`
- Infrastructure exists: `apps/things3-cli/src/mcp/test_harness.rs`

### 2. RELIABILITY.md Documentation

**Status**: Deferred to post-Phase 3  
**Reason**: Better to document patterns after more real-world usage  
**What's Covered**:
- Basic reliability testing in `libs/things3-core/tests/reliability_tests.rs` (8 tests)
- Performance patterns documented in `docs/PERFORMANCE.md`
- Error handling documented in `docs/ERROR_HANDLING.md`

**Plan**:
- Create comprehensive reliability guide after observing real-world patterns
- Document:
  - Connection pool best practices
  - Retry strategies and circuit breakers
  - Graceful degradation patterns
  - Error recovery workflows
  - Memory management guidelines
  - Concurrency patterns

**Related Files**:
- Would create: `docs/RELIABILITY.md`
- Foundation exists: 
  - `libs/things3-core/tests/reliability_tests.rs`
  - `docs/PERFORMANCE.md`
  - `docs/ERROR_HANDLING.md`

### 3. Memory Leak Detection Tests

**Status**: Deferred to post-Phase 3  
**Reason**: Requires external tools (valgrind/miri) not in standard CI  
**What's Covered**:
- Resource cleanup tests in `reliability_tests.rs`
- Proper `Drop` implementations
- Temporary file cleanup

**Plan**:
- Add miri-based leak detection tests
- Optional valgrind testing on Linux
- Document in `docs/PERFORMANCE.md` profiling section

**Tools Required**:
- `cargo +nightly miri test` (for Rust-level leak detection)
- `valgrind` (for system-level leak detection on Linux)
- Heaptrack or Instruments (for profiling)

**Related Files**:
- Would add: `libs/things3-core/tests/memory_leak_tests.rs`
- Infrastructure: Already have `reliability_tests.rs` for resource cleanup

### 4. Enhanced Benchmark Comparison with Statistical Analysis

**Status**: Deferred to post-Phase 3  
**Reason**: Need baseline data from multiple runs first  
**What's Covered**:
- Criterion provides basic statistical analysis
- Benchmarks run in CI and store artifacts
- Manual comparison possible with `--baseline` flag

**Plan**:
- Add automated regression detection (>10% slowdown)
- Statistical significance testing (t-test, Mann-Whitney U)
- Trend analysis over time
- GitHub comment with detailed comparison

**Tools to Add**:
- `critcmp` for benchmark comparison
- Custom GitHub Action for regression detection
- Store baseline results in git or artifacts

**Related Files**:
- Would enhance: `.github/workflows/benchmarks.yml`
- Would add: `tools/scripts/compare-benchmarks.sh`

### 5. Retry Logic and Circuit Breaker Implementation

**Status**: Deferred - evaluate need based on real-world usage  
**Reason**: No evidence of transient failures yet; premature optimization  
**What's Covered**:
- Connection pool handles transient database issues
- Error recovery tested in `reliability_tests.rs`
- Proper error propagation throughout

**When to Implement**:
- If users report transient SQLite lock errors
- If network operations are added (cloud sync, etc.)
- If external service integration is needed

**Plan** (if needed):
- Add `retry` crate for transient failure handling
- Implement exponential backoff
- Add circuit breaker for repeated failures
- Document patterns in `docs/RELIABILITY.md`

**Potential Files**:
- Would add: `libs/things3-core/src/retry.rs`
- Would add: `libs/things3-core/src/circuit_breaker.rs`

## Why These Deferrals Make Sense

### Delivering Value First
Phase 3 successfully delivered:
- ✅ 15+ benchmarks for critical paths
- ✅ 8 comprehensive reliability tests
- ✅ CI integration with regression detection
- ✅ Comprehensive compatibility testing
- ✅ Performance and compatibility documentation

### Avoiding Over-Engineering
- Don't implement solutions for problems we haven't encountered
- Let real-world usage inform reliability patterns
- Gather baseline data before adding complex analysis

### Maintainability
- Smaller, focused PRs are easier to review
- Incremental improvements are easier to maintain
- Can adapt based on actual user needs

## Tracking

Create follow-up issues for each deferred item:
- [ ] Issue: Add MCP server benchmarks
- [x] Issue: Create RELIABILITY.md documentation ✅ Completed
- [ ] Issue: Add memory leak detection tests
- [ ] Issue: Enhance benchmark CI with statistical analysis
- [ ] Issue: Evaluate need for retry logic/circuit breakers

## Timeline

**Short-term** (Next 1-2 months):
1. Gather benchmark baseline data from CI runs
2. Monitor for reliability issues in real-world usage
3. Create `docs/RELIABILITY.md` based on patterns observed

**Medium-term** (2-6 months):
1. Add MCP server benchmarks if performance becomes a concern
2. Implement statistical benchmark comparison
3. Add memory leak detection tests

**Long-term** (6+ months):
1. Evaluate need for retry logic based on user reports
2. Consider circuit breaker if external services are integrated

## References

- Phase 3 Plan: `docs/plans/phase3-performance-reliability.md`
- Existing Tests: `libs/things3-core/tests/reliability_tests.rs`
- Performance Docs: `docs/PERFORMANCE.md`
- Error Handling: `docs/ERROR_HANDLING.md`
- GitHub Issue: #59

