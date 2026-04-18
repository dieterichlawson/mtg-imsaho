"""End-to-end tests for `cmd_merge`.

Uses real git but no agent — merge is a pure git/ticket operation.
Each test sets up a tiny repo with one or two `fix/<id>` branches
that have a real commit on them, runs `cmd_merge`, and asserts
the merge commit landed on master + the ticket transitioned + the
worktree got torn down.
"""

from __future__ import annotations

import argparse
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

from new_pipeline import utils, worktree  # noqa: E402
from new_pipeline.commands import merge as merge_mod  # noqa: E402
from new_pipeline.types import Status, Ticket, TicketError  # noqa: E402


def _git(*args: str, cwd: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args],
        cwd=str(cwd),
        check=True,
        capture_output=True,
        text=True,
    )


def _args(tickets: str) -> argparse.Namespace:
    return argparse.Namespace(tickets=tickets)


class MergeCommandTest(unittest.TestCase):
    """cmd_merge exercised against a real git repo + real worktrees."""

    def setUp(self) -> None:
        """Temp git repo + patched utils paths.

        Mirrors production: tickets are tracked files that get committed
        before merge, worktrees live in a gitignored `.worktrees/` dir.
        Without this, `is_clean()` would always reject because the test
        ticket file would be untracked and the worktree dir untracked too.
        """
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-merge-"))
        _git("init", "-q", "-b", "master", cwd=self.tmp)
        _git("config", "user.email", "t@example.com", cwd=self.tmp)
        _git("config", "user.name", "Merge Test", cwd=self.tmp)
        (self.tmp / "README.md").write_text("baseline\n")
        (self.tmp / ".gitignore").write_text(".worktrees/\n")
        _git("add", "-A", cwd=self.tmp)
        _git("commit", "-q", "-m", "init", cwd=self.tmp)

        tickets = self.tmp / "new_pipeline" / "tickets"
        archive = tickets / "archive"
        tickets.mkdir(parents=True)
        archive.mkdir()
        self.tickets = tickets
        self.archive = archive
        self.worktrees = self.tmp / ".worktrees"

        self._patches = [
            patch.object(utils, "PROJECT_ROOT", self.tmp),
            patch.object(utils, "TICKETS_DIR", tickets),
            patch.object(utils, "ARCHIVE_DIR", archive),
            patch.object(utils, "WORKTREES_DIR", self.worktrees),
        ]
        for p in self._patches:
            p.start()
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _write_fixed_ticket(
        self, ticket_id: str, *, fixed_sha: str = "fakesha",
    ) -> Path:
        """Write a `fixed` ticket file and commit it on master.

        Production tickets are git-tracked, so the merge command's
        cleanliness check expects them committed before merging.
        """
        path = self.tickets / f"{ticket_id}.md"
        path.write_text(textwrap.dedent(
            f"""\
            ---
            id: {ticket_id}
            status: fixed
            card: Fake Card
            test_file: mtg-engine/tests/pipeline_bugs_{ticket_id}.rs
            tested_sha: abc123
            fixed_sha: {fixed_sha}
            ---

            ## Audit Finding
            A fake finding.
            """
        ))
        _git("add", str(path.relative_to(self.tmp)), cwd=self.tmp)
        _git("commit", "-q", "-m", f"ticket {ticket_id}", cwd=self.tmp)
        return path

    def _make_fixed_branch(self, ticket_id: str, content: str) -> str:
        """Create the ticket's worktree, drop a unique commit, return sha."""
        wt = worktree.ensure(ticket_id)
        (wt / f"{ticket_id}.txt").write_text(content)
        _git("add", "-A", cwd=wt)
        _git("commit", "-q", "-m", f"fix {ticket_id}", cwd=wt)
        return _git("rev-parse", "HEAD", cwd=wt).stdout.strip()

    # ── happy path ──────────────────────────────────────────────────

    def test_single_ticket_merges_and_archives(self) -> None:
        """A fixed ticket lands on master, transitions to MERGED, archives."""
        self._write_fixed_ticket("foo-01")
        fix_sha = self._make_fixed_branch("foo-01", "the fix")

        merge_mod.cmd_merge(_args("foo-01"))

        # Ticket: status MERGED, archived, frontmatter has the merge sha.
        t = Ticket.load("foo-01")
        self.assertIs(t.status, Status.MERGED)
        self.assertTrue((self.archive / "foo-01.md").exists())
        self.assertFalse((self.tickets / "foo-01.md").exists())
        self.assertTrue(t.frontmatter.merged_sha)
        self.assertNotEqual(t.frontmatter.merged_sha, fix_sha,
                            "merge --no-ff must produce a NEW merge commit")
        # Worktree + branch torn down.
        self.assertFalse(worktree.dir_for("foo-01").exists())
        self.assertEqual(worktree.branch_head("fix/foo-01"), "")
        # Master HEAD is the archival commit; merge commit is its parent.
        head_parent = _git(
            "rev-parse", "HEAD~1", cwd=self.tmp,
        ).stdout.strip()
        self.assertEqual(head_parent, t.frontmatter.merged_sha)
        # The fix commit's content is now reachable from master.
        log = _git(
            "log", "--oneline", "master", cwd=self.tmp,
        ).stdout
        self.assertIn("fix foo-01", log)
        self.assertIn("Archive ticket foo-01", log)

    def test_multiple_tickets_merge_in_order(self) -> None:
        """Two tickets each get their own merge commit on master."""
        for tid, content in (("a-01", "first"), ("b-01", "second")):
            self._write_fixed_ticket(tid)
            self._make_fixed_branch(tid, content)

        merge_mod.cmd_merge(_args("a-01,b-01"))

        for tid in ("a-01", "b-01"):
            t = Ticket.load(tid)
            self.assertIs(t.status, Status.MERGED)
            self.assertTrue(t.frontmatter.merged_sha)
            self.assertFalse(worktree.dir_for(tid).exists())

        log = _git(
            "log", "--oneline", "master", cwd=self.tmp,
        ).stdout
        self.assertIn("fix a-01", log)
        self.assertIn("fix b-01", log)

    # ── guards ──────────────────────────────────────────────────────

    def test_non_master_branch_raises(self) -> None:
        """If project root isn't on master, the command refuses to start."""
        _git("checkout", "-b", "feature", cwd=self.tmp)
        self._write_fixed_ticket("foo-01")
        self._make_fixed_branch("foo-01", "x")
        with self.assertRaises(TicketError):
            merge_mod.cmd_merge(_args("foo-01"))

    def test_dirty_working_tree_raises(self) -> None:
        """Uncommitted changes at project root → refuse."""
        (self.tmp / "uncommitted.txt").write_text("dirty\n")
        self._write_fixed_ticket("foo-01")
        self._make_fixed_branch("foo-01", "x")
        with self.assertRaises(TicketError):
            merge_mod.cmd_merge(_args("foo-01"))

    def test_non_fixed_ticket_skipped_not_failed(self) -> None:
        """A `new` ticket in the list is skipped; later tickets still merge."""
        # foo-01 is `new` (no fix branch); bar-01 is `fixed`.
        path = self.tickets / "foo-01.md"
        path.write_text(textwrap.dedent(
            """\
            ---
            id: foo-01
            status: new
            card: Fake
            ---
            body
            """
        ))
        _git("add", str(path.relative_to(self.tmp)), cwd=self.tmp)
        _git("commit", "-q", "-m", "ticket foo-01 (new)", cwd=self.tmp)
        self._write_fixed_ticket("bar-01")
        self._make_fixed_branch("bar-01", "real fix")

        merge_mod.cmd_merge(_args("foo-01,bar-01"))

        # foo-01 unchanged, bar-01 merged.
        self.assertIs(Ticket.load("foo-01").status, Status.NEW)
        self.assertIs(Ticket.load("bar-01").status, Status.MERGED)

    def test_missing_branch_raises(self) -> None:
        """Fixed ticket whose branch doesn't exist → TicketError."""
        self._write_fixed_ticket("foo-01")  # no branch created
        with self.assertRaises(TicketError):
            merge_mod.cmd_merge(_args("foo-01"))

    # ── conflict handling ──────────────────────────────────────────

    def test_merge_conflict_aborts_and_stops(self) -> None:
        """Conflict aborts the merge, leaves master clean, stops processing."""
        # Two branches that both modify README at the same line.
        for tid, content in (("a-01", "alpha"), ("b-01", "beta")):
            self._write_fixed_ticket(tid)
            wt = worktree.ensure(tid)
            (wt / "README.md").write_text(content + "\n")
            _git("add", "-A", cwd=wt)
            _git("commit", "-q", "-m", f"fix {tid}", cwd=wt)

        merge_mod.cmd_merge(_args("a-01,b-01"))

        # First merged successfully.
        self.assertIs(Ticket.load("a-01").status, Status.MERGED)
        # Second left in fixed (conflict — operator must intervene).
        self.assertIs(Ticket.load("b-01").status, Status.FIXED)
        # Working tree is clean (merge --abort restored it).
        self.assertTrue(worktree.is_clean())

    # ── arg validation ──────────────────────────────────────────────

    def test_empty_tickets_raises(self) -> None:
        """`--tickets ' , '` → ValueError."""
        with self.assertRaises(ValueError):
            merge_mod.cmd_merge(_args(" , "))


if __name__ == "__main__":
    unittest.main(verbosity=2)
