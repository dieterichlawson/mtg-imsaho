"""Read-only commands: list all tickets, show one by id."""

from __future__ import annotations

import sys

from new_pipeline.types import Status, Ticket, TicketError

# Open statuses first, terminals last.
_DISPLAY_ORDER = [Status.NEW, Status.CLOSED]


def cmd_tickets(args):
    """Entry point for `./new_pipeline/cli.py tickets`."""
    status = Status(args.status) if args.status else None
    tickets = Ticket.list_all(status=status, card=args.card)
    if not tickets:
        print("No tickets found.")
        return

    by_status: dict[Status, list[Ticket]] = {}
    for t in tickets:
        by_status.setdefault(t.status, []).append(t)
    for st in _DISPLAY_ORDER:
        group = by_status.get(st, [])
        if not group:
            continue
        print(f"\n{st.value.upper()} ({len(group)})")
        for t in group:
            print(f"  {t.id:<30} {t.frontmatter.card}")


def cmd_show(args):
    """Entry point for `./new_pipeline/cli.py show`."""
    try:
        t = Ticket.load(args.ticket_id)
    except TicketError:
        print(f"Ticket not found: {args.ticket_id}")
        sys.exit(1)
    print(t.path.read_text())
