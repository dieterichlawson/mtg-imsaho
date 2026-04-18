"""Tests for `validate_fix` — runs real git + scripted cargo on PATH.

A fake `cargo` executable dispatches on `argv[1]` so `cargo check`
and `cargo test` can be scripted independently via
`FAKE_CARGO_CHECK_EXIT` / `FAKE_CARGO_TEST_EXIT` etc. Git is real —
each test initializes a tmp git repo with a baseline commit, commits
a "fix" on top, and asks validate_fix to grade it against the
baseline sha.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline.validate import validate_fix  # noqa: E402

# Fake cargo that dispatches on `sys.argv[1]` ("check" / "test" / …)
# and reads its outcome from FAKE_CARGO_<CMD>_{EXIT,STDOUT,STDERR} env vars.
_FAKE_CARGO = textwrap.dedent(
    """\
    #!/usr/bin/env python3
    import os
    import sys

    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    prefix = f"FAKE_CARGO_{cmd.upper()}_"
    sys.stdout.write(os.environ.get(prefix + "STDOUT", ""))
    sys.stderr.write(os.environ.get(prefix + "STDERR", ""))
    sys.exit(int(os.environ.get(prefix + "EXIT", "0")))
    """
)


def _git(*args: str, cwd: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args],
        cwd=str(cwd),
        check=True,
        capture_output=True,
        text=True,
    )


class ValidateFixTest(unittest.TestCase):
    """Exercise validate_fix against real git + scripted cargo."""

    def setUp(self) -> None:
        """Temp git repo with one baseline commit + fake cargo on PATH."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-vf-"))
        self.worktree = self.tmp / "wt"
        self.worktree.mkdir()
        src = self.worktree / "mtg-engine" / "src"
        src.mkdir(parents=True)

        _git("init", "-q", "-b", "master", cwd=self.worktree)
        _git("config", "user.email", "t@example.com", cwd=self.worktree)
        _git("config", "user.name", "Fix Test", cwd=self.worktree)
        (src / "engine.rs").write_text("// baseline\npub fn run() {}\n")
        _git("add", "-A", cwd=self.worktree)
        _git("commit", "-q", "-m", "baseline", cwd=self.worktree)
        self.tested_sha = _git(
            "rev-parse", "HEAD", cwd=self.worktree,
        ).stdout.strip()

        bindir = self.tmp / "bin"
        bindir.mkdir()
        (bindir / "cargo").write_text(_FAKE_CARGO)
        (bindir / "cargo").chmod(0o755)

        new_path = f"{bindir}:{os.environ.get('PATH', '')}"
        self._patches = [
            patch.dict(os.environ, {"PATH": new_path}, clear=False),
        ]
        for p in self._patches:
            p.start()
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        for k in list(os.environ):
            if k.startswith("FAKE_CARGO_"):
                del os.environ[k]
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _commit_fix(self, rel: str, contents: str) -> None:
        path = self.worktree / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents)
        _git("add", "-A", cwd=self.worktree)
        _git("commit", "-q", "-m", "fix", cwd=self.worktree)

    def _run(self) -> str | None:
        return validate_fix(self.worktree, self.tested_sha)

    # ── happy path ────────────────────────────────────────────────

    def test_clean_fix_validates(self) -> None:
        """Clean diff + cargo ok + clean tree → None."""
        self._commit_fix(
            "mtg-engine/src/engine.rs",
            "// fix\npub fn run() { let _ = 1; }\n",
        )
        # cargo check and cargo test both exit 0 by default.
        self.assertIsNone(self._run())

    # ── banned phrases ────────────────────────────────────────────

    def test_banned_phrase_in_fix_rejects(self) -> None:
        """A TODO introduced by the fix blocks validation."""
        self._commit_fix(
            "mtg-engine/src/engine.rs",
            "// fix\npub fn run() { /* TODO: finish */ }\n",
        )
        result = self._run()
        self.assertIsNotNone(result)
        self.assertIn("banned phrases", result)
        self.assertIn("TODO", result)

    def test_banned_phrase_in_baseline_does_not_reject(self) -> None:
        """Pre-existing banned phrases (outside the fix diff) are ignored."""
        # Swap the baseline for one containing a TODO, re-commit as new
        # baseline, then commit a clean fix on top.
        (self.worktree / "mtg-engine" / "src" / "engine.rs").write_text(
            "// TODO: legacy unrelated\npub fn run() {}\n"
        )
        _git("add", "-A", cwd=self.worktree)
        _git(
            "commit", "-q", "-m", "new baseline with TODO", cwd=self.worktree,
        )
        self.tested_sha = _git(
            "rev-parse", "HEAD", cwd=self.worktree,
        ).stdout.strip()

        # Now commit a clean fix on top.
        self._commit_fix(
            "mtg-engine/src/engine.rs",
            "// TODO: legacy unrelated\npub fn run() { let _ = 1; }\n",
        )
        self.assertIsNone(self._run())

    # ── cargo check ───────────────────────────────────────────────

    def test_cargo_check_warning_rejects(self) -> None:
        """`warning[...]` anywhere in cargo check output → rejected."""
        self._commit_fix(
            "mtg-engine/src/engine.rs", "// fix\npub fn run() {}\n",
        )
        os.environ["FAKE_CARGO_CHECK_EXIT"] = "0"
        os.environ["FAKE_CARGO_CHECK_STDOUT"] = (
            "   Compiling mtg-engine v0.1.0\n"
            "warning[unused_imports]: unused import `Foo`\n"
        )
        result = self._run()
        self.assertIsNotNone(result)
        self.assertIn("compiler warnings", result)

    # ── cargo test ────────────────────────────────────────────────

    def test_cargo_test_nonzero_rejects(self) -> None:
        """`cargo test` exit nonzero → rejected."""
        self._commit_fix(
            "mtg-engine/src/engine.rs", "// fix\npub fn run() {}\n",
        )
        os.environ["FAKE_CARGO_TEST_EXIT"] = "101"
        os.environ["FAKE_CARGO_TEST_STDOUT"] = (
            "test result: FAILED. 1 failed\n"
        )
        result = self._run()
        self.assertIsNotNone(result)
        self.assertIn("cargo test failed", result)

    def test_cargo_test_failed_marker_rejects(self) -> None:
        """Exit 0 but `FAILED` line in output → rejected (defensive)."""
        self._commit_fix(
            "mtg-engine/src/engine.rs", "// fix\npub fn run() {}\n",
        )
        os.environ["FAKE_CARGO_TEST_EXIT"] = "0"
        os.environ["FAKE_CARGO_TEST_STDOUT"] = (
            "ok — some.test\nFAILED other.test\n"
        )
        result = self._run()
        self.assertIsNotNone(result)
        self.assertIn("cargo test failed", result)

    # ── worktree-clean ────────────────────────────────────────────

    def test_dirty_worktree_rejects(self) -> None:
        """Uncommitted changes → rejected."""
        self._commit_fix(
            "mtg-engine/src/engine.rs", "// fix\npub fn run() {}\n",
        )
        # Introduce an uncommitted edit.
        (self.worktree / "mtg-engine" / "src" / "engine.rs").write_text(
            "// forgot to commit this\npub fn run() {}\n"
        )
        result = self._run()
        self.assertIsNotNone(result)
        self.assertIn("uncommitted", result)


if __name__ == "__main__":
    unittest.main(verbosity=2)
