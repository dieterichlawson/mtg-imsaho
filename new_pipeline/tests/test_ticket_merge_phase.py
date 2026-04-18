"""Unit tests for `Ticket.mark_merged`.

Covers the data-model additions only — no git, no command. Writes a
minimal `fixed`-status ticket, calls the mutation, saves, reloads,
and asserts the on-disk state matches plus that the file got moved
to the archive (MERGED is terminal).
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
from new_pipeline.types import Status, Ticket  # noqa: E402


def _write_fixed_ticket(
    tickets_dir: Path, ticket_id: str = "foo-01",
) -> None:
    """Write a minimal fixed-status ticket file for tests to load."""
    path = tickets_dir / f"{ticket_id}.md"
    path.write_text(textwrap.dedent(
        f"""\
        ---
        id: {ticket_id}
        status: fixed
        card: Fake Card
        test_file: mtg-engine/tests/pipeline_bugs_{ticket_id}.rs
        tested_sha: abc123
        fixed_sha: def456789012345678901234567890abcdef0123
        ---

        ## Audit Finding
        A fake finding.

        ## Fix Result

        **Status:** fixed

        Did the thing.
        """
    ))


class TicketMergePhaseTest(unittest.TestCase):
    """Direct tests for `mark_merged` on disk."""

    def setUp(self) -> None:
        """Temp tickets dir, patched into utils."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-mp-"))
        tickets = self.tmp / "tickets"
        archive = tickets / "archive"
        tickets.mkdir()
        self.tickets = tickets
        self.archive = archive
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

    def test_mark_merged_sets_status_and_metadata(self) -> None:
        """mark_merged transitions to MERGED + stamps merged_sha/merged_at."""
        _write_fixed_ticket(self.tickets)
        t = Ticket.load("foo-01")
        t.mark_merged(merged_sha="aaa111bbb222ccc333ddd444eee555fff666aaaa")
        t.save()

        reloaded = Ticket.load("foo-01")
        self.assertIs(reloaded.status, Status.MERGED)
        fm = reloaded.frontmatter
        self.assertEqual(
            fm.merged_sha, "aaa111bbb222ccc333ddd444eee555fff666aaaa",
        )
        self.assertIsInstance(fm.merged_at, datetime)
        # The earlier fixed_sha is preserved so the audit trail isn't lost.
        self.assertEqual(
            fm.fixed_sha, "def456789012345678901234567890abcdef0123",
        )

    def test_merged_is_terminal_auto_archives(self) -> None:
        """MERGED is terminal — save() moves the file into the archive."""
        _write_fixed_ticket(self.tickets, ticket_id="land-01")
        t = Ticket.load("land-01")
        t.mark_merged(merged_sha="abc")
        t.save()
        self.assertTrue(Status.MERGED.is_terminal)
        self.assertFalse((self.tickets / "land-01.md").exists())
        self.assertTrue(
            (self.tickets / "archive" / "land-01.md").exists()
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
