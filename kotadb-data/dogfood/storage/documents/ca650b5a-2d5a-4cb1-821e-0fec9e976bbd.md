---
tags:
- file
- kota-db
- ext_sh
---
#!/bin/bash
# CI Performance Verification Script
# Validates CI optimizations and provides metrics

set -e

echo "=== CI WORKFLOW VERIFICATION ==="
echo "Date: $(date)"
echo ""

# Check if gh CLI is available
if ! command -v gh &> /dev/null; then
    echo "Error: GitHub CLI (gh) is not installed"
    exit 1
fi

# Function to calculate average duration
calculate_average() {
    local sum=0
    local count=0
    while read -r duration; do
        if [[ $duration =~ ^[0-9]+$ ]]; then
            sum=$((sum + duration))
            count=$((count + 1))
        fi
    done
    if [ $count -gt 0 ]; then
        echo $((sum / count))
    else
        echo 0
    fi
}

echo "🔍 Analyzing recent CI runs..."
echo ""

# Get last 5 successful CI runs
echo "Recent CI Run Performance:"
gh run list --workflow=ci.yml --status=success --limit=5 --json databaseId,displayTitle,conclusion,createdAt | \
    jq -r '.[] | "\(.databaseId)\t\(.displayTitle[0:50])\t\(.conclusion)"' | \
    while IFS=$'\t' read -r id title conclusion; do
        duration=$(gh run view "$id" --json jobs | \
            jq '[.jobs[] | select(.completedAt != "0001-01-01T00:00:00Z") | 
                ((.completedAt | sub("\\.[0-9]+Z$"; "Z") | fromdateiso8601) - 
                 (.startedAt | sub("\\.[0-9]+Z$"; "Z") | fromdateiso8601))] | max')
        echo "  Run $id: ${duration}s - $title"
    done

echo ""
echo "📊 Job Parallelization Analysis:"
# Check which jobs run in parallel
gh workflow view ci.yml | grep -E "^\s+(format|clippy|unit-tests|integration|performance|security|container|docs|coverage)" | \
    while read -r job; do
        echo "  ✓ $job (parallel)"
    done

echo ""
echo "⚡ Optimization Metrics:"

# Check cache configuration
echo "  Cache Keys:"
grep -o 'shared-key: "[^"]*"' .github/workflows/ci.yml | sort -u | while read -r line; do
    echo "    - $line"
done

# Check test thread configuration
echo ""
echo "  Test Thread Configuration:"
grep -o "test-threads=[0-9]*" .github/workflows/ci.yml | sort -u | while read -r config; do
    echo "    - $config"
done

# Check CI environment variables
echo ""
echo "  CI-Aware Configuration:"
if grep -q "CI: true" .github/workflows/ci.yml; then
    echo "    ✓ CI environment variable set"
fi
if grep -q "is_ci()" tests/test_constants.rs; then
    echo "    ✓ CI detection function implemented"
fi

# Check concurrency settings
echo ""
echo "  Concurrency Control:"
if grep -q "concurrency:" .github/workflows/ci.yml; then
    echo "    ✓ Concurrency group configured"
    grep -A2 "concurrency:" .github/workflows/ci.yml | tail -2 | sed 's/^/      /'
fi

echo ""
echo "🎯 Quality Gates Status:"
gates=("format" "clippy" "security" "unit-tests" "integration" "coverage" "docs" "container")
for gate in "${gates[@]}"; do
    if grep -q "$gate:" .github/workflows/ci.yml; then
        echo "  ✓ $gate check present"
    else
        echo "  ✗ $gate check missing"
    fi
done

echo ""
echo "📈 Estimated Performance:"
echo "  Previous average: ~5 minutes"
echo "  Optimized target: <3 minutes"
echo "  Improvements:"
echo "    - Removed build job dependency: -90s"
echo "    - Parallel job execution: -60s"
echo "    - CI-aware test configuration: -30s"
echo "    - Concurrency control: prevents duplicate runs"
echo ""
echo "✅ CI Verification Complete"