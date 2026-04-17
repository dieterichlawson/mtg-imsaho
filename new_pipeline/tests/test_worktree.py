"""Tests for `new_pipeline.worktree` against a real temp git repo.

Each test initializes a fresh git repo with one commit, patches
`utils.PROJECT_ROOT` + `utils.WORKTREES_DIR` to point there, then
exercises the module's helpers. No mocking of subprocess — we want
real git semantics.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import utils, worktree  # noqa: E402


def _git(*args: str, cwd: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args],
        cwd=str(cwd),
        check=True,
        capture_output=True,
        text=True,
    )


class WorktreeTest(unittest.TestCase):
    """Exercise worktree helpers against a real git repo in a tmp dir."""

    def setUp(self) -> None:
        """Init a real git repo at `self.repo` with one commit on master."""
        self.repo = Path(tempfile.mkdtemp(prefix="new-pipeline-wt-"))
        _git("init", "-q", "-b", "master", cwd=self.repo)
        _git("config", "user.email", "t@example.com", cwd=self.repo)
        _git("config", "user.name", "Worktree Test", cwd=self.repo)
        (self.repo / "README.md").write_text("baseline\n")
        _git("add", "-A", cwd=self.repo)
        _git("commit", "-q", "-m", "init", cwd=self.repo)

        self._patches = [
            patch.object(utils, "PROJECT_ROOT", self.repo),
            patch.object(utils, "WORKTREES_DIR", self.repo / ".worktrees"),
        ]
        for p in self._patches:
            p.start()
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        # Use `git worktree list` + `worktree.remove` isn't necessary here
        # because shutil.rmtree nukes everything including the .git files.
        shutil.rmtree(self.repo, ignore_errors=True)

    def _branches(self) -> set[str]:
        out = _git(
            "for-each-ref", "--format=%(refname:short)", "refs/heads/",
            cwd=self.repo,
        ).stdout.strip().splitlines()
        return set(out)

    # ── pure helpers ──────────────────────────────────────────────

    def test_dir_for_is_deterministic(self) -> None:
        """dir_for maps ticket ids to a predictable path under WORKTREES_DIR."""
        p = worktree.dir_for("olivia_voldaren-01")
        self.assertEqual(p.parent, utils.WORKTREES_DIR)
        self.assertEqual(p.name, "fix-olivia_voldaren-01")

    def test_branch_for_follows_the_convention(self) -> None:
        """Branch names are `fix/<ticket_id>`."""
        self.assertEqual(
            worktree.branch_for("olivia_voldaren-01"),
            "fix/olivia_voldaren-01",
        )

    # ── ensure ────────────────────────────────────────────────────

    def test_ensure_creates_worktree_and_branch(self) -> None:
        """First call to ensure creates the directory + branch."""
        wt = worktree.ensure("foo-01")
        self.assertTrue(wt.exists())
        self.assertTrue((wt / ".git").exists())  # linked worktree marker
        self.assertIn("fix/foo-01", self._branches())

    def test_ensure_is_idempotent(self) -> None:
        """Calling ensure twice returns the same path without re-creating."""
        first = worktree.ensure("foo-01")
        # Drop a marker file; a recreate would wipe it.
        (first / "marker.txt").write_text("hello")
        second = worktree.ensure("foo-01")
        self.assertEqual(first, second)
        self.assertTrue((second / "marker.txt").exists())

    # ── remove ────────────────────────────────────────────────────

    def test_remove_deletes_worktree_and_branch(self) -> None:
        """After remove, both the directory and the branch are gone."""
        wt = worktree.ensure("foo-01")
        self.assertTrue(wt.exists())
        self.assertIn("fix/foo-01", self._branches())

        worktree.remove("foo-01")
        self.assertFalse(wt.exists())
        self.assertNotIn("fix/foo-01", self._branches())

    def test_remove_is_idempotent_when_absent(self) -> None:
        """Remove on a ticket that never had a worktree is a no-op."""
        worktree.remove("never-existed-01")  # must not raise
        self.assertFalse(worktree.dir_for("never-existed-01").exists())

    # ── branch_head ───────────────────────────────────────────────

    def test_branch_head_returns_sha_for_existing_branch(self) -> None:
        """branch_head returns a 40-char sha for a branch that exists."""
        worktree.ensure("foo-01")
        sha = worktree.branch_head("fix/foo-01")
        self.assertEqual(len(sha), 40)
        self.assertTrue(all(c in "0123456789abcdef" for c in sha))

    def test_branch_head_returns_empty_when_missing(self) -> None:
        """branch_head returns '' for a non-existent branch."""
        self.assertEqual(worktree.branch_head("fix/never-existed-99"), "")


if __name__ == "__main__":
    unittest.main(verbosity=2)
