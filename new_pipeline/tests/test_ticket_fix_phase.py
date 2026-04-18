"""Unit tests for `Ticket.mark_fixed` / `Ticket.mark_fix_failed`.

Exercises the data-model additions directly — no command, no agent,
no cargo. Each test writes a minimal `tested`-status ticket to disk,
loads it, calls the mutation, saves, reloads, and asserts the on-disk
state matches.
"""

from __future__ import annotations

import shutil
import sys
import tempfile
import textwrap
import unittest
from datetime import datetime
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import utils  # noqa: E402
from new_pipeline.types import (  # noqa: E402
    FixReport,
    FixStatus,
    Status,
    Ticket,
)


def _write_tested_ticket(
    tickets_dir: Path, ticket_id: str = "foo-01",
) -> None:
    """Write a minimal tested-status ticket file for tests to load."""
    path = tickets_dir / f"{ticket_id}.md"
    path.write_text(textwrap.dedent(
        f"""\
        ---
        id: {ticket_id}
        status: tested
        card: Fake Card
        test_file: mtg-engine/tests/pipeline_bugs_{ticket_id}.rs
        tested_sha: abc123
        ---

        ## Audit Finding
        A fake finding.

        ## Tests

        ### test_scenario
        Scenario: exercise the bug.
        """
    ))


class TicketFixPhaseTest(unittest.TestCase):
    """Direct tests for mark_fixed / mark_fix_failed on disk."""

    def setUp(self) -> None:
        """Temp tickets dir, patched into utils."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-fp-"))
        tickets = self.tmp / "tickets"
        archive = tickets / "archive"
        tickets.mkdir()
        self.tickets = tickets
        self._patches = [
            patch.object(utils, "TICKETS_DIR", tickets),
            patch.object(utils, "ARCHIVE_DIR", archive),
        ]
        for p in self._patches:
            p.start()
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)

    # ── mark_fixed ────────────────────────────────────────────────

    def test_mark_fixed_sets_status_and_all_metadata(self) -> None:
        """mark_fixed transitions status + stamps every fix_* frontmatter.

        The mutator doesn't touch the body; the caller pairs it with
        `append_body_section(report.to_result_section())` when it wants the
        outcome recorded in the body too.
        """
        _write_tested_ticket(self.tickets)
        t = Ticket.load("foo-01")

        report = FixReport(
            status=FixStatus.FIXED,
            description="Added missing lifelink trigger on first-strike.",
        )
        t.mark_fixed(
            run_id="2026-04-17-foo-01-fix",
            model="opus",
            tokens=1234,
            duration=42,
            fixed_sha="def456789012345678901234567890abcdef0123",
        )
        t.append_body_section(report.to_result_section())
        t.save()

        reloaded = Ticket.load("foo-01")
        self.assertIs(reloaded.status, Status.FIXED)
        fm = reloaded.frontmatter
        self.assertEqual(fm.fix_run_id, "2026-04-17-foo-01-fix")
        self.assertEqual(fm.fix_model, "opus")
        self.assertEqual(fm.fix_tokens, 1234)
        self.assertEqual(fm.fix_duration, 42)
        self.assertEqual(
            fm.fixed_sha, "def456789012345678901234567890abcdef0123",
        )
        self.assertIsInstance(fm.fixed_at, datetime)
        self.assertIsNone(fm.fix_failed_at)
        self.assertIn("## Fix Result", reloaded.body)
        self.assertIn("Added missing lifelink trigger", reloaded.body)

    def test_mark_fixed_does_not_touch_body(self) -> None:
        """Mutator updates state only; body stays untouched until append."""
        _write_tested_ticket(self.tickets, ticket_id="foo-02")
        t = Ticket.load("foo-02")
        body_before = t.body
        t.mark_fixed(
            run_id="r", model="m", tokens=1, duration=1, fixed_sha="s",
        )
        self.assertEqual(t.body, body_before)

    def test_mark_fixed_does_not_save(self) -> None:
        """The mutator is pure — caller must call .save() to persist."""
        _write_tested_ticket(self.tickets, ticket_id="foo-03")
        t = Ticket.load("foo-03")
        t.mark_fixed(
            run_id="r", model="m", tokens=1, duration=1, fixed_sha="s",
        )
        # No .save() yet → disk still says `tested`.
        self.assertIs(Ticket.load("foo-03").status, Status.TESTED)

    def test_fixed_is_not_terminal_stays_in_active(self) -> None:
        """FIXED is not terminal — the file stays in the active dir."""
        _write_tested_ticket(self.tickets, ticket_id="stay-01")
        t = Ticket.load("stay-01")
        t.mark_fixed(
            run_id="r", model="m", tokens=1, duration=1, fixed_sha="s",
        )
        t.save()
        self.assertFalse(Status.FIXED.is_terminal)
        self.assertTrue((self.tickets / "stay-01.md").exists())
        self.assertFalse(
            (self.tickets / "archive" / "stay-01.md").exists()
        )

    # ── mark_fix_failed ───────────────────────────────────────────

    def test_mark_fix_failed_sets_status_and_post_mortem_metadata(
        self,
    ) -> None:
        """mark_fix_failed transitions + stamps metadata (no fixed_sha)."""
        _write_tested_ticket(self.tickets, ticket_id="giveup-01")
        t = Ticket.load("giveup-01")

        report = FixReport(
            status=FixStatus.FAILED,
            description=(
                "Tried X. Got stuck on Y. Would need Z before this "
                "could be expressed."
            ),
        )
        t.mark_fix_failed(
            run_id="2026-04-17-giveup-01-fix",
            model="opus",
            tokens=567,
            duration=12,
        )
        t.append_body_section(report.to_result_section())
        t.save()

        reloaded = Ticket.load("giveup-01")
        self.assertIs(reloaded.status, Status.FIX_FAILED)
        fm = reloaded.frontmatter
        self.assertEqual(fm.fix_run_id, "2026-04-17-giveup-01-fix")
        self.assertEqual(fm.fix_tokens, 567)
        self.assertEqual(fm.fix_duration, 12)
        self.assertIsInstance(fm.fix_failed_at, datetime)
        # No successful fix → no fixed_sha / fixed_at
        self.assertEqual(fm.fixed_sha, "")
        self.assertIsNone(fm.fixed_at)
        # Post-mortem appended
        self.assertIn("## Fix Result", reloaded.body)
        self.assertIn("Got stuck on Y", reloaded.body)

    def test_fix_failed_is_terminal_auto_archives(self) -> None:
        """FIX_FAILED is terminal; retry mints a new ticket instead."""
        _write_tested_ticket(self.tickets, ticket_id="retryable-01")
        t = Ticket.load("retryable-01")
        t.mark_fix_failed(
            run_id="r", model="m", tokens=1, duration=1,
        )
        t.save()
        self.assertTrue(Status.FIX_FAILED.is_terminal)
        self.assertFalse((self.tickets / "retryable-01.md").exists())
        self.assertTrue(
            (self.tickets / "archive" / "retryable-01.md").exists()
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
