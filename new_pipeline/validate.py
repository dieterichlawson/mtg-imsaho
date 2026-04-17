"""Cargo-based validators for agent-written tests.

`validate_test` runs `cargo test` against a single test function the
test-writer agent claimed it wrote, and returns a typed verdict. A
test "passing" validation (ok=True) means cargo compiled it and it
failed with an assertion — proof that the bug is real and reproducible.
"""

from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

# Phrases that shouldn't appear in a real test file — placeholders,
# hedging language, anything suggesting the test is half-baked.
_BANNED_PHRASES = re.compile(
    r"\bTODO|\bFIXME|further investigation|would need to|beyond the scope|"
    r"\bfor now\b|in the future|\bleft as\b|not sure|might need",
    re.IGNORECASE,
)

_ASSERTION_PATTERN = re.compile(r"assert!|assert_eq!|assert_ne!")

_ASSERTION_FAILURE_PATTERN = re.compile(
    r"assertion.*failed|assert.*failed|left.*right",
)


@dataclass
class TestValidation:
    """Verdict from running `validate_test` on one test function."""

    ok: bool
    reason: str
    output: str = ""  # trimmed cargo output tail, non-empty on failure


def validate_test(
    worktree: Path, test_file_rel: str, test_name: str,
) -> TestValidation:
    """Check that `test_name` in `test_file_rel` compiles AND fails.

    Runs under `worktree` (usually a per-ticket git worktree). "Passing"
    validation means the test compiles and fails with an assertion —
    the bug is real. A test that cargo-passes means the scenario is a
    false positive; a test that panics without asserting is malformed.
    """
    test_file = worktree / test_file_rel
    if not test_file.exists():
        return TestValidation(
            False, f"test file does not exist: {test_file_rel}",
        )

    contents = test_file.read_text()
    if _BANNED_PHRASES.search(contents):
        return TestValidation(
            False,
            "test file contains banned placeholder phrases "
            "(TODO / FIXME / 'would need to' etc.)",
        )
    if not _ASSERTION_PATTERN.search(contents):
        return TestValidation(False, "no assertions found (need at least 1)")

    bin_name = Path(test_file_rel).stem  # e.g. pipeline_bugs_<tid>
    r = subprocess.run(
        [
            "cargo", "test", "-p", "mtg-engine",
            "--test", bin_name,
            "--", test_name, "--exact",
        ],
        capture_output=True,
        text=True,
        cwd=str(worktree),
    )
    output = (r.stdout or "") + (r.stderr or "")

    if r.returncode == 0:
        return TestValidation(
            False,
            "test passes — bug appears to be a false positive",
            output=output[-2000:],
        )
    if _ASSERTION_FAILURE_PATTERN.search(output):
        return TestValidation(
            True,
            "test compiles and fails with assertion",
            output=output[-1500:],
        )
    if "panicked at" in output:
        return TestValidation(
            False,
            "test fails with panic, not assertion error",
            output=output[-1500:],
        )
    return TestValidation(
        False,
        "test fails but unclear failure type",
        output=output[-1500:],
    )
