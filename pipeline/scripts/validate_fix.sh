#!/bin/bash
# Validate fixer output.
# Usage: validate_fix.sh [test_name]
# Exit codes: 0 = fix valid, 1 = fix invalid

set -euo pipefail

TEST_NAME="${1:-}"

echo "=== Validating fix ==="

# 1. Check no test files were modified
echo "--- Checking for test file modifications ---"
MODIFIED_TESTS=$(git diff --name-only | grep "^mtg-engine/tests/" || true)
if [[ -n "$MODIFIED_TESTS" ]]; then
    echo "REJECTED: Test files were modified (not allowed):"
    echo "$MODIFIED_TESTS"
    exit 1
fi
echo "No test files modified: OK"

# 2. Check for banned phrases in changed code
echo "--- Checking for banned phrases in diff ---"
BANNED_PATTERNS="TODO|FIXME|hack|workaround|temporary"
DIFF_ADDITIONS=$(git diff -- 'mtg-engine/src/' | grep '^+' | grep -v '^+++' || true)
if echo "$DIFF_ADDITIONS" | grep -qEi "$BANNED_PATTERNS"; then
    echo "REJECTED: Banned phrases found in code changes:"
    echo "$DIFF_ADDITIONS" | grep -Ei "$BANNED_PATTERNS"
    exit 1
fi
echo "No banned phrases: OK"

# 3. Check compilation with zero warnings
echo "--- Checking compilation (zero warnings) ---"
CARGO_CHECK=$(cargo check 2>&1)
if echo "$CARGO_CHECK" | grep -q "warning\["; then
    echo "REJECTED: Compiler warnings found:"
    echo "$CARGO_CHECK" | grep "warning\["
    exit 1
fi
echo "Compilation clean: OK"

# 4. Run target test (if specified)
if [[ -n "$TEST_NAME" ]]; then
    echo "--- Running target test: $TEST_NAME ---"
    TARGET_OUTPUT=$(cargo test -- "$TEST_NAME" 2>&1)
    if echo "$TARGET_OUTPUT" | grep -q "FAILED"; then
        echo "REJECTED: Target test still fails"
        echo "$TARGET_OUTPUT" | grep -E "FAILED|assertion" | head -5
        exit 1
    fi
    # Check at least one test ran and passed
    if ! echo "$TARGET_OUTPUT" | grep -q "1 passed"; then
        echo "REJECTED: Target test not found or did not pass"
        exit 1
    fi
    echo "Target test passes: OK"
fi

# 5. Run full test suite
echo "--- Running full test suite ---"
TEST_OUTPUT=$(cargo test 2>&1)
if echo "$TEST_OUTPUT" | grep -q "FAILED"; then
    echo "REJECTED: Some tests fail:"
    echo "$TEST_OUTPUT" | grep "FAILED\|failures:" | head -20
    exit 1
fi

echo "Full test suite passes: OK"

# 6. Worktree must be clean — all changes committed
echo "--- Checking worktree is clean (all changes committed) ---"
UNCOMMITTED=$(git status --porcelain)
if [[ -n "$UNCOMMITTED" ]]; then
    echo "REJECTED: worktree has uncommitted or untracked files:"
    echo "$UNCOMMITTED"
    echo ""
    echo "Commit all of your changes (including the test file, if untracked)"
    echo "before writing your staging output:"
    echo "    git add -A && git commit -m '<message>'"
    exit 1
fi
echo "Worktree clean: OK"

echo ""
echo "=== FIX VALIDATED ==="
exit 0
