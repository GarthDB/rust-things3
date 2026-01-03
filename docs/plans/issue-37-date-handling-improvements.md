# Issue #37: Improve Date Handling and Conversion Accuracy

**Status**: In Progress  
**Branch**: `37-improve-date-handling-and-conversion`  
**Assignee**: AI Assistant  
**Created**: 2025-01-02

## Overview

Improve date conversion accuracy, add robust validation, and enhance error handling for date-related operations throughout the Things 3 integration.

## Goals

1. Add comprehensive date validation utilities
2. Improve error handling for invalid dates
3. Add logical date validation (deadline after start date, etc.)
4. Handle edge cases properly (null dates, far future/past dates)
5. Ensure date conversion accuracy matches Things 3 UI
6. Add extensive testing for date operations

## Current State Analysis

### Date Conversion Functions (in `database/core.rs`)
```rust
pub(crate) fn things_date_to_naive_date(seconds_since_2001: i64) -> Option<NaiveDate>
pub fn naive_date_to_things_timestamp(date: NaiveDate) -> i64
```

**Current Limitations:**
- Returns `None` for `seconds_since_2001 <= 0` (could be valid zero date)
- No upper bound checking (overflow potential)
- No error messages, just `None`
- Unwraps could panic in edge cases

### Date Fields Used Throughout
- `Task`: `start_date`, `deadline`, `created`, `modified`, `stop_date`
- `Project`: `start_date`, `deadline`, `created`, `modified`
- `CreateTaskRequest`: `start_date`, `deadline`
- `UpdateTaskRequest`: `start_date`, `deadline`

**Current State:**
- ✅ Basic conversion working
- ❌ No validation that deadline > start_date
- ❌ No validation of date ranges
- ❌ No explicit error messages for invalid dates
- ❌ No timezone awareness documented

## Implementation Plan

### Phase 1: Date Validation Module

**File**: `libs/things3-core/src/database/date_utils.rs` (NEW)

Create a dedicated date validation and conversion module with comprehensive utilities.

**Functions to Implement:**

1. **`validate_date_range(start: Option<NaiveDate>, deadline: Option<NaiveDate>) -> Result<(), DateValidationError>`**
   - Ensure deadline is not before start date
   - Return descriptive errors

2. **`is_valid_things_timestamp(seconds: i64) -> bool`**
   - Check if timestamp is within reasonable range
   - Things 3 released in 2009, so dates before 2000 might be suspicious
   - Check for overflow (year 2100+)

3. **`safe_things_date_to_naive_date(seconds_since_2001: i64) -> Result<NaiveDate, DateConversionError>`**
   - Replace `Option` with `Result` for better error messages
   - Handle zero/negative values explicitly
   - Check for overflow
   - Validate result is reasonable

4. **`safe_naive_date_to_things_timestamp(date: NaiveDate) -> Result<i64, DateConversionError>`**
   - Validate date is not before 2001
   - Check for overflow in calculation
   - Return descriptive errors

5. **`validate_task_dates(request: &CreateTaskRequest) -> Result<(), DateValidationError>`**
   - Validate start_date and deadline relationship
   - Could be used before creating/updating tasks

6. **`validate_project_dates(request: &CreateProjectRequest) -> Result<(), DateValidationError>`**
   - Same for projects

**Error Types:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum DateConversionError {
    #[error("Date is before Things 3 epoch (2001-01-01)")]
    BeforeEpoch,
    
    #[error("Date timestamp {0} is invalid or overflow")]
    InvalidTimestamp(i64),
    
    #[error("Date is too far in the future (after 2100)")]
    TooFarFuture,
    
    #[error("Date conversion overflow")]
    Overflow,
}

#[derive(Debug, thiserror::Error)]
pub enum DateValidationError {
    #[error("Deadline {deadline} cannot be before start date {start_date}")]
    DeadlineBeforeStartDate {
        start_date: NaiveDate,
        deadline: NaiveDate,
    },
    
    #[error("Date conversion failed: {0}")]
    ConversionFailed(#[from] DateConversionError),
}
```

### Phase 2: Integrate Validation into Database Operations

**Files to Update:**
- `libs/things3-core/src/database/core.rs`
- `libs/things3-core/src/database/validators.rs`

**Changes:**

1. **Update `create_task` to validate dates:**
   ```rust
   pub async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<Uuid> {
       // Validate dates before insertion
       validate_task_dates(&request)?;
       // ... rest of implementation
   }
   ```

2. **Update `update_task` to validate dates:**
   ```rust
   pub async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()> {
       // If updating dates, validate them
       if request.start_date.is_some() || request.deadline.is_some() {
           // Get current task to merge dates
           let current_task = self.get_task_by_uuid(&request.uuid).await?;
           let final_start = request.start_date.or(current_task.start_date);
           let final_deadline = request.deadline.or(current_task.deadline);
           validate_date_range(final_start, final_deadline)?;
       }
       // ... rest of implementation
   }
   ```

3. **Update `create_project` and `update_project` similarly**

4. **Replace internal use of `things_date_to_naive_date` with `safe_things_date_to_naive_date`**
   - Update `TaskRowMapper` in `mappers.rs`
   - Log warnings for dates that fail validation but continue gracefully

### Phase 3: Enhanced Error Messages

**Files to Update:**
- `libs/things3-core/src/error.rs`

**Changes:**

1. **Add date-specific error variants to `ThingsError`:**
   ```rust
   #[error("Date validation failed: {0}")]
   DateValidation(#[from] DateValidationError),
   
   #[error("Date conversion failed: {0}")]
   DateConversion(#[from] DateConversionError),
   ```

2. **Update MCP error handling to provide user-friendly messages**

### Phase 4: Add Date Helper Functions

**Additional Utilities in `date_utils.rs`:**

1. **`format_date_for_display(date: Option<NaiveDate>) -> String`**
   - Consistent date formatting across the app
   - Handle None gracefully

2. **`parse_date_from_string(s: &str) -> Result<NaiveDate, DateConversionError>`**
   - Support multiple formats (ISO 8601, etc.)
   - Better error messages than chrono's default

3. **`is_date_in_past(date: NaiveDate) -> bool`**
   - Helper for business logic

4. **`is_date_in_future(date: NaiveDate) -> bool`**
   - Helper for business logic

5. **`add_days(date: NaiveDate, days: i64) -> Result<NaiveDate, DateConversionError>`**
   - Safe date arithmetic with overflow checking

### Phase 5: Comprehensive Testing

**Test File**: `libs/things3-core/tests/date_handling_tests.rs` (NEW)

**Test Categories (20+ tests):**

1. **Conversion Tests (8 tests)**
   - `test_things_date_conversion_basic` - Normal dates
   - `test_things_date_conversion_epoch` - 2001-01-01
   - `test_things_date_conversion_negative` - Negative timestamps
   - `test_things_date_conversion_zero` - Zero timestamp
   - `test_things_date_conversion_far_future` - Year 2100+
   - `test_things_date_conversion_overflow` - MAX values
   - `test_naive_date_to_things_before_epoch` - Pre-2001 dates
   - `test_round_trip_conversion` - Date -> timestamp -> date

2. **Validation Tests (8 tests)**
   - `test_validate_deadline_after_start` - Valid case
   - `test_validate_deadline_before_start` - Invalid case
   - `test_validate_same_date` - Edge case
   - `test_validate_only_start_date` - No deadline
   - `test_validate_only_deadline` - No start date
   - `test_validate_no_dates` - Both None
   - `test_validate_task_dates` - Full request validation
   - `test_validate_project_dates` - Project validation

3. **Edge Case Tests (6 tests)**
   - `test_handle_null_dates_gracefully`
   - `test_handle_invalid_timestamp_gracefully`
   - `test_date_formatting_consistency`
   - `test_date_parsing_multiple_formats`
   - `test_date_arithmetic_overflow`
   - `test_date_comparison_edge_cases`

4. **Integration Tests (4 tests)**
   - `test_create_task_with_invalid_dates` - Should fail validation
   - `test_update_task_deadline_before_start` - Should fail
   - `test_create_task_with_valid_dates` - Should succeed
   - `test_update_task_dates_successfully` - Should succeed

**Test File**: `apps/things3-cli/tests/mcp_date_handling_tests.rs` (NEW)

**MCP Integration Tests (6 tests):**
1. `test_create_task_mcp_date_validation_error`
2. `test_update_task_mcp_date_validation_error`
3. `test_create_project_mcp_date_validation`
4. `test_date_error_messages_are_clear`
5. `test_create_task_with_valid_dates_mcp`
6. `test_update_task_dates_mcp`

### Phase 6: Documentation

**Files to Update:**

1. **`THINGS3_DATABASE_ANALYSIS.md`**
   - Add detailed section on date handling
   - Document the epoch (2001-01-01)
   - Explain integer vs real timestamps

2. **`docs/ARCHITECTURE.md`**
   - Add date validation flow diagram
   - Document error handling strategy

3. **API Documentation**
   - Add doc comments to all new functions
   - Include examples

## Date Field Analysis

### Things 3 Database Schema

From `THINGS3_DATABASE_ANALYSIS.md`:

```sql
-- INTEGER fields (seconds since 2001-01-01)
startDate INTEGER      -- Start date for tasks/projects
deadline INTEGER       -- Deadline for tasks/projects

-- REAL fields (seconds since 2001-01-01, but stored as float)
creationDate REAL      -- Creation timestamp
userModificationDate REAL  -- Last modification timestamp
stopDate REAL          -- Completion timestamp (for tasks)
```

**Key Insights:**
- Start dates and deadlines use INTEGER
- Timestamps use REAL (f64)
- All use same epoch: 2001-01-01 00:00:00 UTC
- Null values allowed

### Observed Edge Cases

From issue description:
- Some dates showing as "2005-03-17" - need to verify if correct
- Need to compare with Things 3 UI for accuracy

**Test Strategy:**
- Create test tasks in actual Things 3 app
- Query them via MCP
- Verify dates match exactly

## Implementation Order

1. ✅ Create `date_utils.rs` with validation functions and error types
2. ✅ Add comprehensive unit tests in `date_handling_tests.rs`
3. ✅ Integrate validation into `create_task`, `update_task`, `create_project`, `update_project`
4. ✅ Update `ThingsError` with date-specific variants
5. ✅ Add MCP integration tests in `mcp_date_handling_tests.rs`
6. ✅ Update documentation
7. ✅ Run full test suite
8. ✅ Manual testing with real Things 3 database
9. ✅ Create PR

## Success Criteria

- ✅ All new validation functions implemented
- ✅ All date operations validate input before DB operations
- ✅ Clear error messages for all date validation failures
- ✅ 26+ new tests passing (20 unit + 6 integration)
- ✅ No regressions in existing functionality
- ✅ Documentation updated
- ✅ CI/CD pipeline passes

## Notes

- Focus on making date errors **actionable** - users should know exactly what's wrong
- Preserve backward compatibility - existing tasks with weird dates should still load
- Consider adding a "strict mode" flag for validation in the future
- Could add date timezone configuration in future enhancement
- Validation should be **defensive but not breaking** - log warnings for suspicious dates but don't fail operations unless clearly invalid

## Migration Strategy

Since this adds validation that didn't exist before:
- Existing data might have invalid dates (deadline before start)
- Validation should only apply to NEW operations
- Consider adding a `--validate-dates` flag to check existing data
- Log warnings for suspicious dates on read operations, but don't fail

## Future Enhancements (Out of Scope)

- Timezone support and configuration
- Natural language date parsing ("tomorrow", "next week")
- Date recurrence patterns
- Calendar integration
- Date conflict detection across projects

