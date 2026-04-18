"""Cargo-based validators for agent work.

Both functions return `str | None`:
    - None  → validated, nothing to complain about
    - str   → a ready-to-use retry note (short reason + cargo tail
              where relevant) to feed back to the agent

`validate_test` runs `cargo test` against a single test function the
test-writer claimed it wrote; "validated" means the test compiles and
fails with an assertion — proof the bug is real.

`validate_fix` runs `cargo check`, `cargo test`, and a banned-phrase
scan over the fix diff; "validated" means the fix introduces no
warnings, all tests pass at HEAD, the worktree is clean, and the diff
contains no placeholder language.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

# ── Test-writer validator ─────────────────────────────────────────

# Phrases that shouldn't appear in a real test file — placeholders,
# hedging language, anything suggesting the test is half-baked.
_TEST_BANNED = re.compile(
    r"\bTODO|\bFIXME|further investigation|would need to|beyond the scope|"
    r"\bfor now\b|in the future|\bleft as\b|not sure|might need",
    re.IGNORECASE,
)

_ASSERTION_PATTERN = re.compile(r"assert!|assert_eq!|assert_ne!")

_ASSERTION_FAILURE_PATTERN = re.compile(
    r"assertion.*failed|assert.*failed|left.*right",
)


def validate_test(
    worktree: Path, test_file_rel: str, test_name: str,
) -> str | None:
    """Validate one test. Returns None on success, retry-note on failure.

    "Success" means the test compiles and fails with an assertion — the
    bug is real. A test that cargo-passes is a false positive. A test
    that panics without asserting is malformed.
    """
    test_file = worktree / test_file_rel
    if not test_file.exists():
        return f"test file does not exist: {test_file_rel}"

    contents = test_file.read_text()
    if _TEST_BANNED.search(contents):
        return (
            "test file contains banned placeholder phrases "
            "(TODO / FIXME / 'would need to' etc.)"
        )
    if not _ASSERTION_PATTERN.search(contents):
        return "no assertions found (need at least 1)"

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
        return (
            "test passes — bug appears to be a false positive\n"
            f"{output[-2000:]}"
        )
    if _ASSERTION_FAILURE_PATTERN.search(output):
        return None  # bug confirmed
    if "panicked at" in output:
        return (
            "test fails with panic, not assertion error\n"
            f"{output[-1500:]}"
        )
    return f"test fails but unclear failure type\n{output[-1500:]}"


# ── Fixer validator ───────────────────────────────────────────────

# Phrases that shouldn't appear in fix diffs — placeholder/hedge
# language that signals a half-done change.
_FIX_BANNED = re.compile(
    r"\bTODO|\bFIXME|\bhack|\bworkaround|\btemporary",
    re.IGNORECASE,
)


def validate_fix(worktree: Path, tested_sha: str) -> str | None:
    """Validate a fix against `tested_sha` baseline.

    Four checks in order, first failure short-circuits:
        1. Banned phrases in the diff (`tested_sha..HEAD`).
        2. `cargo check` — any warnings fail.
        3. `cargo test` — nonzero exit or "FAILED" marker fails.
        4. `git status --porcelain` — must be clean.

    Returns None if all pass; a retry-note string otherwise.
    """
    # 1. Banned phrases in what the fix actually added (not pre-
    # existing text in untouched files).
    diff = subprocess.run(
        ["git", "diff", f"{tested_sha}..HEAD", "--", "mtg-engine/src/"],
        capture_output=True,
        text=True,
        cwd=str(worktree),
    ).stdout
    additions = "\n".join(
        ln for ln in diff.splitlines()
        if ln.startswith("+") and not ln.startswith("+++")
    )
    if _FIX_BANNED.search(additions):
        return (
            "banned phrases (TODO/FIXME/hack/workaround/temporary) "
            f"in fix diff:\n{additions[-1000:]}"
        )

    # 2. cargo check — zero warnings.
    check = subprocess.run(
        ["cargo", "check"],
        capture_output=True,
        text=True,
        cwd=str(worktree),
    )
    check_output = (check.stdout or "") + (check.stderr or "")
    if "warning[" in check_output:
        return f"compiler warnings present:\n{check_output[-2000:]}"

    # 3. Full test suite — exit 0 AND no "FAILED" lines.
    tests = subprocess.run(
        ["cargo", "test"],
        capture_output=True,
        text=True,
        cwd=str(worktree),
    )
    test_output = (tests.stdout or "") + (tests.stderr or "")
    if tests.returncode != 0 or "FAILED" in test_output:
        return (
            f"cargo test failed (exit {tests.returncode}):\n"
            f"{test_output[-2000:]}"
        )

    # 4. Worktree clean.
    status = subprocess.run(
        ["git", "status", "--porcelain"],
        capture_output=True,
        text=True,
        cwd=str(worktree),
    ).stdout.strip()
    if status:
        return (
            "worktree has uncommitted or untracked files — commit "
            f"before declaring the fix done:\n{status}"
        )

    return None
