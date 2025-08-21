# Test Failure Investigation - Issue #282

## Purpose
Investigating test suite failures discovered after CI timeout fixes in PR #281.

## Local Test Results

### Performance Regression Test
- **Status**: FAILED (Actual performance regression detected)
- **Command**: `cargo test --release --features bench performance_regression_test`
- **Issue**: Search performance degraded from 10k→100k entries
- **Details**: Time ratio 5.92x when expected ~3.32x (O(log n) growth)
- **Root Cause**: Performance regression in search operations at scale

### With CI Environment
- **Status**: PASSING  
- **Command**: `CI=true cargo test --release --features bench performance_regression_test`
- **Duration**: 0.02s
- **Note**: CI uses smaller test sizes (100, 1k, 10k vs 100, 1k, 10k, 100k)

### Other Test Suites
- **Stress Tests**: PASSING (1.27s)
- **System Tests**: PASSING (5.10s)  
- **Query Tests**: PASSING (0.01s)

## Root Cause Analysis
The test failure is **legitimate** - there's a real performance regression in search operations:
- Size 10k→100k (10x increase) shows 5.92x time increase
- Expected for O(log n): ~3.32x time increase
- Actual regression: 78% slower than expected

## Next Steps
1. ✅ Identify the specific performance regression
2. ✅ Investigate B+ tree search implementation 
3. ✅ Fix the performance issue (redundant tree traversal in insert)
4. ✅ Verify fix resolves the regression
5. 🔄 Update CI to catch this earlier

## Solution Applied
**Root Cause**: `insert_into_tree` was performing redundant tree traversals:
1. `search_in_tree(&root, &key)` to check if key exists (line 272)
2. `insert_recursive(...)` to perform the actual insert (line 279)

**Fix**: Created `insert_recursive_with_exists_check()` that tracks key existence during the single tree traversal, eliminating the redundant search.

**Performance Impact**:
- Before: 775ns per operation at 100k entries (1,289,143 ops/sec)
- After: 254ns per operation at 100k entries (3,930,555 ops/sec)  
- **Improvement**: 3.05x faster (67% performance boost)
- Growth ratio improved from 5.92x to 2.1x (closer to expected 3.32x for O(log n))

## Notes
- Unit tests all pass (175 passed, 6 ignored)
- This is not a test infrastructure issue - it's a real performance bug
- CI masks the issue due to smaller test sizes