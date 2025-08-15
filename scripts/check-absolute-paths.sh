#!/bin/bash

# Pre-commit check for absolute paths in test code
# This prevents accidentally using absolute paths that would fail validation

set -e

echo "Checking for absolute paths in test code..."

# Files to check (test files and test modules in source files)
TEST_FILES=$(find . -name "*.rs" -type f | grep -E "(tests/|src/)" | grep -v target)

# Pattern to detect absolute paths in test code
# Looks for paths starting with / or C:\ in string literals
# Excludes common false positives like URLs, regex patterns, and temp directories

FOUND_ISSUES=0
PROBLEMATIC_FILES=""

for file in $TEST_FILES; do
    # Check if file contains test code
    if grep -q "#\[test\]" "$file" || grep -q "#\[cfg(test)\]" "$file" || echo "$file" | grep -q "tests/"; then
        # Look for absolute paths in string literals, excluding:
        # - Comments (lines starting with //)
        # - URLs (containing ://)
        # - Temp directories (/tmp, /var/folders)
        # - Regex patterns (in Regex::new or similar)
        # - Documentation comments
        
        if grep -E '(ValidatedPath::new|\.path|DocumentBuilder.*path|"path":|path:\s*String).*"/' "$file" | \
           grep -v '://' | \
           grep -v '/tmp' | \
           grep -v '/var/folders' | \
           grep -v '^[[:space:]]*//' | \
           grep -v 'Regex::' | \
           grep -v 'assert.*is_err' | \
           grep -v 'should.*reject' | \
           grep -v 'dangerous_paths' | \
           grep -v 'invalid.*path' | \
           grep -v 'absolute_paths' >/dev/null 2>&1; then
            
            echo "❌ Found potential absolute path in: $file"
            echo "   Lines with issues:"
            grep -n -E '(ValidatedPath::new|\.path|DocumentBuilder.*path|"path":|path:\s*String).*"/' "$file" | \
                grep -v '://' | \
                grep -v '/tmp' | \
                grep -v '/var/folders' | \
                grep -v '^[[:space:]]*//' | \
                grep -v 'Regex::' | \
                grep -v 'assert.*is_err' | \
                grep -v 'should.*reject' | \
                grep -v 'dangerous_paths' | \
                grep -v 'invalid.*path' | \
                grep -v 'absolute_paths' | \
                head -3 || true
            
            FOUND_ISSUES=$((FOUND_ISSUES + 1))
            PROBLEMATIC_FILES="$PROBLEMATIC_FILES $file"
        fi
        
        # Also check for Windows-style absolute paths
        if grep -E '(ValidatedPath::new|\.path|DocumentBuilder.*path|"path":|path:\s*String).*"[A-Z]:\\' "$file" | \
           grep -v 'assert.*is_err' | \
           grep -v 'should.*reject' | \
           grep -v 'dangerous_paths' | \
           grep -v 'invalid.*path' >/dev/null 2>&1; then
            
            echo "❌ Found Windows absolute path in: $file"
            FOUND_ISSUES=$((FOUND_ISSUES + 1))
            PROBLEMATIC_FILES="$PROBLEMATIC_FILES $file"
        fi
    fi
done

if [ $FOUND_ISSUES -gt 0 ]; then
    echo ""
    echo "⚠️  Found $FOUND_ISSUES file(s) with absolute paths in test code"
    echo ""
    echo "Absolute paths are not allowed in KotaDB for security reasons."
    echo "Please use relative paths instead:"
    echo "  ❌ '/test/doc.md' → ✅ 'test/doc.md'"
    echo "  ❌ '/data/storage' → ✅ 'data/storage'"
    echo ""
    echo "Files to fix:"
    for f in $PROBLEMATIC_FILES; do
        echo "  - $f"
    done
    echo ""
    echo "Note: This check ignores:"
    echo "  - Paths in security tests that intentionally test absolute path rejection"
    echo "  - Temporary directories (/tmp, /var/folders)"
    echo "  - URLs and file:// schemes"
    echo "  - Comments and documentation"
    echo ""
    exit 1
else
    echo "✅ No absolute paths found in test code"
fi