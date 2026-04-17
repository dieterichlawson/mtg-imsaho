"""Python ports of `validate_test.sh` and `validate_fix.sh`.

Both scripts wrap `cargo` + some banned-phrase / assertion counting.
Keeping this in Python means failure details, retry notes, and log output
are all in the same language as the pipeline orchestrator.
"""
from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

# Banned phrases — detectable in text before we even try to compile.
TEST_BANNED_PATTERN = re.compile(
    r"\bTODO|\bFIXME|further investigation|would need to|beyond the scope|"
    r"\bfor now\b|in the future|\bleft as\b|not sure|might need",
    re.IGNORECASE)

FIX_BANNED_PATTERN = re.compile(
    r"\bTODO|\bFIXME|\bhack|\bworkaround|\btemporary",
    re.IGNORECASE)

ASSERTION_PATTERN = re.compile(r"assert!|assert_eq!|assert_ne!")


@dataclass
class TestValidation:
    """Result of running `validate_test` on one written test."""

    ok: bool
    reason: str
    output: str = ""   # trimmed cargo output on failure

    @property
    def is_confirmed_bug(self) -> bool:
        """True when the test compiles and fails — demonstrating the bug."""
        return self.ok


@dataclass
class FixValidation:
    """Result of running `validate_fix` against the whole worktree."""

    ok: bool
    reason: str
    output: str = ""


# ── Test-writer validation ──────────────────────────────────────────

def validate_test(worktree: Path, test_file_rel: str, test_name: str) -> TestValidation:
    """Check the test compiles AND fails with an assertion error.

    Passing = "the bug is real and demonstrable." A test that compiles
    and fails is a confirmed bug; one that passes against current code
    is a false-positive scenario and we report it as rejected.
    """
    test_file = worktree / test_file_rel
    if not test_file.exists():
        return TestValidation(False, f"test file does not exist: {test_file_rel}")

    contents = test_file.read_text()
    if TEST_BANNED_PATTERN.search(contents):
        return TestValidation(False,
            "test file contains banned placeholder phrases (TODO / FIXME / "
            "'would need to' etc.)")
    if not ASSERTION_PATTERN.search(contents):
        return TestValidation(False, "no assertions found (need at least 1)")

    bin_name = Path(test_file_rel).stem  # pipeline_bugs_<tid>
    r = subprocess.run(
        ["cargo", "test", "-p", "mtg-engine", "--test", bin_name,
         "--", test_name, "--exact"],
        capture_output=True, text=True, cwd=str(worktree))
    output = (r.stdout or "") + (r.stderr or "")

    if r.returncode == 0:
        # Cargo exited 0 → the test passed → the bug is a false positive.
        return TestValidation(
            False, "test passes — bug appears to be a false positive",
            output=output[-2000:])

    if _has_assertion_failure(output):
        return TestValidation(True, "test compiles and fails with assertion",
                              output=output[-1500:])
    if "panicked at" in output:
        return TestValidation(
            False, "test fails with panic, not assertion error",
            output=output[-1500:])
    return TestValidation(False, "test fails but unclear failure type",
                          output=output[-1500:])


def _has_assertion_failure(output: str) -> bool:
    return bool(re.search(
        r"assertion.*failed|assert.*failed|left.*right", output))


# ── Fixer validation ────────────────────────────────────────────────

def validate_fix(worktree: Path) -> FixValidation:
    """Check the fix: banned phrases in diff → compile clean → full test
    suite passes → worktree is clean. All four must hold.
    """
    # 1. Banned phrases in the *diff* (not the whole tree — pre-existing
    # hack/workaround strings in untouched files shouldn't reject).
    diff = subprocess.run(
        ["git", "diff", "--", "mtg-engine/src/"],
        capture_output=True, text=True, cwd=str(worktree)).stdout
    additions = "\n".join(
        ln for ln in diff.splitlines()
        if ln.startswith("+") and not ln.startswith("+++"))
    if FIX_BANNED_PATTERN.search(additions):
        return FixValidation(False,
            "banned phrases (TODO/FIXME/hack/workaround/temporary) in diff",
            output=additions[-1000:])

    # 2. cargo check — zero warnings.
    check = subprocess.run(["cargo", "check"], capture_output=True,
                           text=True, cwd=str(worktree))
    check_output = (check.stdout or "") + (check.stderr or "")
    if "warning[" in check_output:
        return FixValidation(False, "compiler warnings present",
                             output=check_output[-2000:])

    # 3. Full test suite — exit 0 AND no "FAILED" lines.
    tests = subprocess.run(["cargo", "test"], capture_output=True,
                           text=True, cwd=str(worktree))
    test_output = (tests.stdout or "") + (tests.stderr or "")
    if tests.returncode != 0 or "FAILED" in test_output:
        return FixValidation(
            False, f"cargo test failed (exit {tests.returncode})",
            output=test_output[-2000:])

    # 4. Worktree clean.
    status = subprocess.run(
        ["git", "status", "--porcelain"], capture_output=True,
        text=True, cwd=str(worktree)).stdout.strip()
    if status:
        return FixValidation(False,
            "worktree has uncommitted or untracked files — commit before "
            "declaring the fix done",
            output=status)

    return FixValidation(True, "fix validated")
