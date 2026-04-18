"""End-to-end tests for `cmd_retry`.

Uses real git (for the worktree rename) but no agent/cargo — retry
is a ticket-file-and-git-operation only, no subprocesses beyond
what worktree.rename calls.

Covers both retry paths:
  - FIX_FAILED → new TESTED ticket, worktree renamed, body inherits
    up through `## Test Run Results` plus `## Previous attempt`.
  - COULD_NOT_CONFIRM → new NEW ticket, worktree removed, body
    inherits up through `## Tests` plus `## Previous attempt`.
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
from new_pipeline.commands import retry as retry_mod  # noqa: E402
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


_FIX_FAILED_BODY = textwrap.dedent(
    """\
    ## Audit Finding
    A fake finding.

    ## Tests

    ### scenario_a
    Scenario: exercise the bug.

    ## Test Run Results

    - **scenario_a** — confirmed
      - test fn: `test_scenario_a`

    ## Fix Result

    **Status:** failed

    Tried X, got stuck on Y, would need engine change Z.
    """
)

_COULD_NOT_CONFIRM_BODY = textwrap.dedent(
    """\
    ## Audit Finding
    A fake finding.

    ## Tests

    ### scenario_a
    Scenario: exercise the bug.

    ## Test Run Results

    - **scenario_a** — needs_engine_work
      - explanation: would need `combat::apply_damage_with_source()`
    """
)


class RetryCommandTest(unittest.TestCase):
    """cmd_retry exercised against real git + scripted ticket files."""

    def setUp(self) -> None:
        """Temp git repo + per-test patches for utils paths."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-retry-"))
        _git("init", "-q", "-b", "master", cwd=self.tmp)
        _git("config", "user.email", "t@example.com", cwd=self.tmp)
        _git("config", "user.name", "Retry Test", cwd=self.tmp)
        (self.tmp / "README.md").write_text("baseline\n")
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

    def _write_archived_ticket(
        self,
        ticket_id: str,
        status: str,
        body: str,
        *,
        extra_frontmatter: str = "",
    ) -> Path:
        """Write a ticket file directly into the archive dir."""
        path = self.archive / f"{ticket_id}.md"
        header_parts = [
            "---",
            f"id: {ticket_id}",
            f"status: {status}",
            "card: Fake Card",
        ]
        if extra_frontmatter:
            header_parts.append(extra_frontmatter.rstrip())
        header_parts.append("---")
        path.write_text("\n".join(header_parts) + "\n\n" + body)
        return path

    # ── FIX_FAILED → TESTED ──────────────────────────────────────

    def test_fix_failed_retry_mints_tested_with_worktree(self) -> None:
        """FIX_FAILED retry mints TESTED successor + renames worktree."""
        # Set up a worktree on fix/foo-01 with a committed test file.
        wt_old = worktree.ensure("foo-01")
        test_rel = "mtg-engine/tests/pipeline_bugs_foo_01.rs"
        (wt_old / test_rel).parent.mkdir(parents=True, exist_ok=True)
        (wt_old / test_rel).write_text(
            "#[test]\nfn scenario_a() { assert!(false, \"bug\"); }\n"
        )
        _git("add", "-A", cwd=wt_old)
        _git("commit", "-q", "-m", "tests", cwd=wt_old)
        tested_sha = _git(
            "rev-parse", "HEAD", cwd=wt_old,
        ).stdout.strip()

        # Archived FIX_FAILED ticket with inherited test metadata.
        self._write_archived_ticket(
            "foo-01",
            "fix_failed",
            _FIX_FAILED_BODY,
            extra_frontmatter=(
                "test_run_id: 2026-04-17-foo-01-test\n"
                "test_model: opus\n"
                "test_tokens: 150\n"
                "test_duration: 5\n"
                f"test_file: {test_rel}\n"
                f"tested_sha: {tested_sha}\n"
                "tested_at: 2026-04-17T12:00:00Z\n"
                "fix_run_id: 2026-04-17-foo-01-fix\n"
                "fix_model: opus\n"
                "fix_failed_at: 2026-04-17T13:00:00Z\n"
            ),
        )

        retry_mod.cmd_retry(_args("foo-01"))

        # Successor ticket minted.
        new_id = "foo-02"
        new = Ticket.load(new_id)
        self.assertIs(new.status, Status.TESTED)
        self.assertEqual(new.frontmatter.retry_of, "foo-01")
        # Test-phase metadata carried forward.
        self.assertEqual(new.frontmatter.test_run_id, "2026-04-17-foo-01-test")
        self.assertEqual(new.frontmatter.test_tokens, 150)
        self.assertEqual(new.frontmatter.test_file, test_rel)
        self.assertEqual(new.frontmatter.tested_sha, tested_sha)
        self.assertIsNotNone(new.frontmatter.tested_at)
        # Body inherited up through ## Test Run Results + ## Previous attempt.
        self.assertIn("## Audit Finding", new.body)
        self.assertIn("## Test Run Results", new.body)
        self.assertIn("## Previous attempt (foo-01)", new.body)
        self.assertIn("got stuck on Y", new.body)
        # The original `## Fix Result` heading should NOT appear.
        self.assertNotIn("## Fix Result", new.body)

        # Worktree renamed.
        self.assertFalse(worktree.dir_for("foo-01").exists())
        self.assertTrue(worktree.dir_for("foo-02").exists())
        # Branch renamed.
        self.assertEqual(worktree.branch_head("fix/foo-01"), "")
        self.assertEqual(worktree.branch_head("fix/foo-02"), tested_sha)

        # Old ticket untouched in archive.
        old = Ticket.load("foo-01")
        self.assertIs(old.status, Status.FIX_FAILED)
        self.assertIn("## Fix Result", old.body)

    # ── COULD_NOT_CONFIRM → NEW ──────────────────────────────────

    def test_could_not_confirm_retry_mints_new_and_tears_worktree(
        self,
    ) -> None:
        """COULD_NOT_CONFIRM retry mints NEW successor; worktree removed."""
        wt_old = worktree.ensure("bar-01")
        # Drop a placeholder file so the worktree isn't empty.
        (wt_old / "scratch.rs").write_text("// partial work\n")
        _git("add", "-A", cwd=wt_old)
        _git("commit", "-q", "-m", "partial", cwd=wt_old)

        self._write_archived_ticket(
            "bar-01", "could_not_confirm", _COULD_NOT_CONFIRM_BODY,
        )

        retry_mod.cmd_retry(_args("bar-01"))

        new = Ticket.load("bar-02")
        self.assertIs(new.status, Status.NEW)
        self.assertEqual(new.frontmatter.retry_of, "bar-01")
        # Body: audit finding + tests, plus previous attempt.
        self.assertIn("## Audit Finding", new.body)
        self.assertIn("## Tests", new.body)
        self.assertIn("## Previous attempt (bar-01)", new.body)
        self.assertIn("apply_damage_with_source", new.body)
        # Old `## Test Run Results` heading is gone.
        self.assertNotIn("## Test Run Results", new.body)

        # Worktree for old ticket torn down; no new worktree yet
        # (test phase will create one).
        self.assertFalse(worktree.dir_for("bar-01").exists())
        self.assertFalse(worktree.dir_for("bar-02").exists())

    # ── invalid source status ────────────────────────────────────

    def test_retry_on_tested_raises(self) -> None:
        """Retrying a non-terminal ticket (e.g. tested) is an error."""
        (self.tickets / "active-01.md").write_text(textwrap.dedent(
            """\
            ---
            id: active-01
            status: tested
            card: Fake Card
            ---

            body
            """
        ))
        with self.assertRaises(TicketError):
            retry_mod.cmd_retry(_args("active-01"))

    # ── recursive retry ───────────────────────────────────────────

    def test_retry_of_retry_accumulates_previous_attempts(self) -> None:
        """A-01 → A-02 → A-03 each layer a `## Previous attempt`."""
        # Simulate A-02 already being a retry of A-01 that itself later
        # landed in COULD_NOT_CONFIRM. Body has the A-01 previous-attempt
        # section from the first retry, plus A-02's own test results.
        body_a02 = textwrap.dedent(
            """\
            ## Audit Finding
            finding.

            ## Tests

            ### s
            Scenario: s.

            ## Test Run Results

            - **s** — rejected
              - explanation: still can't verify

            ## Previous attempt (a-01)

            Tried earlier; rejected back then.
            """
        )
        self._write_archived_ticket("a-02", "could_not_confirm", body_a02)

        retry_mod.cmd_retry(_args("a-02"))

        new = Ticket.load("a-03")
        self.assertIs(new.status, Status.NEW)
        self.assertEqual(new.frontmatter.retry_of, "a-02")
        # Both previous-attempt sections are present.
        self.assertIn("## Previous attempt (a-01)", new.body)
        self.assertIn("## Previous attempt (a-02)", new.body)
        self.assertIn("still can't verify", new.body)
        self.assertIn("Tried earlier", new.body)

    # ── arg validation ───────────────────────────────────────────

    def test_empty_tickets_raises(self) -> None:
        """`--tickets ' , '` → ValueError."""
        with self.assertRaises(ValueError):
            retry_mod.cmd_retry(_args(" , "))


if __name__ == "__main__":
    unittest.main(verbosity=2)
