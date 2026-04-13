#!/bin/bash
# Validate test writer output.
# Usage: validate_test.sh <test_file> <test_name>
# Exit codes: 0 = confirmed (test fails), 1 = rejected, 2 = blocked, 3 = error

set -euo pipefail

TEST_FILE="${1:?Usage: validate_test.sh <test_file> <test_name>}"
TEST_NAME="${2:?Usage: validate_test.sh <test_file> <test_name>}"

echo "=== Validating test: $TEST_NAME in $TEST_FILE ==="

# 1. Check file exists
if [[ ! -f "$TEST_FILE" ]]; then
    echo "REJECTED: Test file does not exist: $TEST_FILE"
    exit 1
fi

# 2. Check for banned phrases
BANNED_PATTERNS="TODO|FIXME|further investigation|would need to|beyond the scope|for now|in the future|left as|not sure|might need"
if grep -qEi "$BANNED_PATTERNS" "$TEST_FILE"; then
    echo "REJECTED: Banned phrases found in test file:"
    grep -nEi "$BANNED_PATTERNS" "$TEST_FILE"
    exit 1
fi

# 3. Check minimum assertions
ASSERT_COUNT=$(grep -c 'assert\!\|assert_eq!\|assert_ne!' "$TEST_FILE" || true)
if [[ "$ASSERT_COUNT" -lt 1 ]]; then
    echo "REJECTED: No assertions found in test file (need at least 1)"
    exit 1
fi
echo "Assertions found: $ASSERT_COUNT"

# 4. Derive the test binary name from the test file
TEST_BIN=$(basename "$TEST_FILE" .rs)

# 5. Compile and run the specific test file only (avoids pre-existing errors in other files)
echo "--- Compiling and running test: $TEST_NAME (from $TEST_BIN) ---"
TEST_OUTPUT=$(cargo test -p mtg-engine --test "$TEST_BIN" -- "$TEST_NAME" --exact 2>&1) && {
    echo "REJECTED: Test passes — bug appears to be a false positive"
    echo "$TEST_OUTPUT"
    exit 1
}

# Test failed (good!). Check it's an assertion failure, not a panic.
if echo "$TEST_OUTPUT" | grep -q "assertion.*failed\|assert.*failed\|left.*right"; then
    echo "CONFIRMED: Test compiles and fails with assertion error"
    echo "--- Failure output ---"
    echo "$TEST_OUTPUT" | grep -A5 "assertion\|FAILED\|left.*right" | head -20
    exit 0
elif echo "$TEST_OUTPUT" | grep -q "panicked at"; then
    echo "REJECTED: Test fails with panic, not assertion error:"
    echo "$TEST_OUTPUT" | grep -A3 "panicked at" | head -10
    exit 1
else
    echo "REJECTED: Test fails but unclear failure type:"
    echo "$TEST_OUTPUT" | tail -20
    exit 1
fi
