"""Unit + integration tests for create / list / show / close.

No agents, no worktrees — just the disk-backed ticket lifecycle.
"""

from __future__ import annotations

import argparse
import shutil
import sys
import tempfile
import textwrap
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

# Make `new_pipeline.*` importable regardless of invocation cwd.
_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import utils  # noqa: E402
from new_pipeline.commands import close as close_mod  # noqa: E402
from new_pipeline.commands import show as show_mod  # noqa: E402
from new_pipeline.types import (  # noqa: E402
    CloseReason,
    LifecycleEvent,
    Status,
    Ticket,
    TicketError,
    next_status,
)


def _write_ticket(
    ticket_id: str,
    *,
    status: Status = Status.NEW,
    card: str = "Fake Card",
    body: str = "## Description\n\nBody.\n",
    extra_frontmatter: str = "",
) -> Path:
    """Mint a minimal ticket file on disk for tests to load/close."""
    target_dir = (
        utils.ARCHIVE_DIR if status.is_terminal else utils.TICKETS_DIR
    )
    target_dir.mkdir(parents=True, exist_ok=True)
    path = target_dir / f"{ticket_id}.md"
    header = textwrap.dedent(
        f"""\
        ---
        id: {ticket_id}
        status: {status.value}
        card: {card}
        """
    )
    if extra_frontmatter:
        header += extra_frontmatter.rstrip() + "\n"
    header += "---\n\n"
    path.write_text(header + body)
    return path


class TicketLifecycleTest(unittest.TestCase):
    """Ticket read/save + the close command against a temp workspace."""

    def setUp(self) -> None:
        """Point utils.TICKETS_DIR at a fresh temp dir."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-"))
        tickets = self.tmp / "tickets"
        archive = tickets / "archive"
        tickets.mkdir()
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

    # ── load / list ────────────────────────────────────────────────

    def test_load_roundtrips_frontmatter_and_body(self) -> None:
        """load(id) parses frontmatter into typed fields + body unchanged."""
        _write_ticket(
            "olivia-01",
            card="Olivia Voldaren",
            body="## Description\n\nFlying only once.\n",
            extra_frontmatter="created: 2026-01-01T00:00:00Z\n",
        )
        t = Ticket.load("olivia-01")
        self.assertEqual(t.id, "olivia-01")
        self.assertEqual(t.status, Status.NEW)
        self.assertEqual(t.frontmatter.card, "Olivia Voldaren")
        self.assertIn("Flying only once.", t.body)
        self.assertEqual(
            t.frontmatter.created,
            datetime(2026, 1, 1, tzinfo=timezone.utc),
        )

    def test_load_raises_when_missing(self) -> None:
        """load(id) raises TicketError if the file isn't there."""
        with self.assertRaises(TicketError):
            Ticket.load("not-a-ticket-99")

    def test_list_all_sees_active_and_archive(self) -> None:
        """list_all walks both directories and honors status filters."""
        _write_ticket("live-01")
        _write_ticket("dead-01", status=Status.CLOSED)

        ids = {t.id for t in Ticket.list_all()}
        self.assertEqual(ids, {"live-01", "dead-01"})

        only_closed = {t.id for t in Ticket.list_all(status=Status.CLOSED)}
        self.assertEqual(only_closed, {"dead-01"})

        only_new = {t.id for t in Ticket.list_all(status=Status.NEW)}
        self.assertEqual(only_new, {"live-01"})

    def test_unknown_frontmatter_keys_round_trip(self) -> None:
        """Keys Frontmatter doesn't model survive save/load verbatim.

        Lets hand-authored fields stay on disk when the pipeline rewrites
        a ticket.
        """
        _write_ticket(
            "future-01", extra_frontmatter="custom_note: stays-around\n",
        )
        t = Ticket.load("future-01")
        self.assertEqual(t.frontmatter.extras, {"custom_note": "stays-around"})
        t.save()
        raw = (utils.TICKETS_DIR / "future-01.md").read_text()
        self.assertIn("custom_note: stays-around", raw)

    def test_datetime_fields_round_trip(self) -> None:
        """closed_at set to a datetime parses back to the same datetime."""
        _write_ticket("dt-01")
        t = Ticket.load("dt-01")
        t.frontmatter.closed_at = datetime(
            2026, 4, 17, 12, 0, 0, tzinfo=timezone.utc,
        )
        t.save()

        reloaded = Ticket.load("dt-01")
        self.assertEqual(
            reloaded.frontmatter.closed_at,
            datetime(2026, 4, 17, 12, 0, 0, tzinfo=timezone.utc),
        )

    # ── state machine + close command ──────────────────────────────

    def test_next_status_abandoned_from_open(self) -> None:
        """`abandoned` event on an open status → `closed`."""
        self.assertIs(
            next_status(Status.NEW, LifecycleEvent.ABANDONED),
            Status.CLOSED,
        )

    def test_next_status_rejects_terminal(self) -> None:
        """Abandoning an already-terminal ticket is a ValueError."""
        with self.assertRaises(ValueError):
            next_status(Status.CLOSED, LifecycleEvent.ABANDONED)

    def test_cmd_close_archives_ticket(self) -> None:
        """`close` moves the file into archive/ and stamps the frontmatter."""
        _write_ticket("gone-01")
        active = utils.TICKETS_DIR / "gone-01.md"
        archive = utils.ARCHIVE_DIR / "gone-01.md"
        self.assertTrue(active.exists())

        close_mod.cmd_close(
            argparse.Namespace(ticket_id="gone-01", note="not worth it")
        )

        self.assertFalse(active.exists())
        self.assertTrue(archive.exists())

        t = Ticket.load("gone-01")
        self.assertIs(t.status, Status.CLOSED)
        self.assertEqual(
            t.frontmatter.closed_reason, CloseReason.ABANDONED.value,
        )
        self.assertEqual(t.frontmatter.closed_note, "not worth it")
        self.assertIsInstance(t.frontmatter.closed_at, datetime)

    # ── show / tickets commands ────────────────────────────────────

    def test_cmd_tickets_groups_by_status(self) -> None:
        """`tickets` groups + prints every ticket on disk."""
        _write_ticket("a-01")
        _write_ticket("b-01", status=Status.CLOSED)

        with patch("builtins.print") as pr:
            show_mod.cmd_tickets(
                argparse.Namespace(status=None, card=None)
            )
        printed = "\n".join(
            " ".join(str(a) for a in call.args) for call in pr.call_args_list
        )
        self.assertIn("NEW (1)", printed)
        self.assertIn("a-01", printed)
        self.assertIn("CLOSED (1)", printed)
        self.assertIn("b-01", printed)

    def test_cmd_show_missing_ticket_exits(self) -> None:
        """`show` on a missing id prints an error and exits non-zero."""
        with self.assertRaises(SystemExit) as ctx:
            show_mod.cmd_show(argparse.Namespace(ticket_id="nope-99"))
        self.assertEqual(ctx.exception.code, 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
