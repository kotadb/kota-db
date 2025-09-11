---
tags:
- file
- kota-db
- ext_sh
---
#!/usr/bin/env bash
# CI Local Testing Script
# Tests the CI workflow locally to validate optimizations

set -e

echo "=== KotaDB CI Local Test Script ==="
echo "This script simulates CI workflow locally for validation"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
export CI=true
export RUST_LOG=error
export RUST_TEST_THREADS=4
export CARGO_BUILD_JOBS=4

# Track timings
START_TIME=$(date +%s)

# Function to print timing
print_timing() {
    local job_name=$1
    local start=$2
    local end=$(date +%s)
    local duration=$((end - start))
    echo -e "${GREEN}✓${NC} $job_name completed in ${duration}s"
}

# Function to run test group
run_test_group() {
    local name=$1
    shift
    local start=$(date +%s)
    echo -e "\n${YELLOW}Running: $name${NC}"
    
    if cargo test "$@" --release --features bench > /dev/null 2>&1; then
        print_timing "$name" $start
        return 0
    else
        echo -e "${RED}✗${NC} $name FAILED"
        return 1
    fi
}

echo -e "\n${YELLOW}Step 1: Building release binaries...${NC}"
build_start=$(date +%s)
cargo build --release --all-features
cargo test --release --no-run --all-features
print_timing "Build" $build_start

echo -e "\n${YELLOW}Step 2: Running test suites...${NC}"

# Unit tests
run_test_group "Unit Tests" --lib

# Core integration
run_test_group "Integration (Core)" \
    --test 'file_storage*' \
    --test 'builder*' \
    --test 'validated_types*' \
    --test 'data_integrity*' \
    --test 'unicode_handling*' \
    --test 'storage_index*'

# Index integration
run_test_group "Integration (Index)" \
    --test 'primary_index*' \
    --test 'btree*' \
    --test 'test_btree*' \
    --test 'query_routing*' \
    --test 'bulk_operations*'

# Stress tests part 1
run_test_group "Integration (Stress 1)" \
    --test 'concurrent_stress*' \
    --test 'concurrent_access*'

# Stress tests part 2
run_test_group "Integration (Stress 2)" \
    --test 'index_stress*' \
    --test 'chaos*' \
    --test 'adversarial*' \
    --test 'complexity_comparison*' \
    --test '*performance*'

# System integration
run_test_group "Integration (System)" \
    --test 'cli_path*' \
    --test 'http_server*' \
    --test 'observability*' \
    --test 'production_configuration*' \
    --test 'system_resilience*'

# Calculate total time
END_TIME=$(date +%s)
TOTAL_TIME=$((END_TIME - START_TIME))
MINUTES=$((TOTAL_TIME / 60))
SECONDS=$((TOTAL_TIME % 60))

echo -e "\n${GREEN}=== CI Test Complete ===${NC}"
echo -e "Total time: ${MINUTES}m ${SECONDS}s"

if [ $TOTAL_TIME -lt 300 ]; then
    echo -e "${GREEN}✓ Target met: Under 5 minutes!${NC}"
else
    echo -e "${YELLOW}⚠ Target missed: Over 5 minutes (${MINUTES}m ${SECONDS}s)${NC}"
fi

echo -e "\n${YELLOW}Recommendations:${NC}"
echo "1. If stress tests are slow, consider further splitting"
echo "2. Monitor memory usage during tests"
echo "3. Check for hanging tests with 'cargo test -- --test-threads=1 --nocapture'"
echo "4. Use 'just test-perf' to validate performance regression tests"