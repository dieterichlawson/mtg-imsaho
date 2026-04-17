"""Close a ticket as abandoned."""

from __future__ import annotations

from new_pipeline.types import Ticket


def cmd_close(args):
    """Entry point for `./new_pipeline/cli.py close`."""
    t = Ticket.load(args.ticket_id)
    t.close(note=args.note)
    t.save()
    print(f"Closed {t.id}")
