"""Read-only commands: tickets, show, status, report."""

from __future__ import annotations

import subprocess
import sys

from pipeline import utils
from pipeline.models import Ticket
from pipeline.state import Status

# Status order used for grouped display.
_DISPLAY_ORDER = [
    Status.NEW,
    Status.TESTED,
    Status.FIXED,
    Status.FIX_FAILED,
    Status.FALSE_POSITIVE,
    Status.SHIPPED,
    Status.CLOSED,
]


def cmd_tickets(args):
    """Entry point for `./pipeline/cli.py tickets`."""
    status = Status(args.status) if args.status else None
    tickets = Ticket.list_all(status=status, card=args.card)
    if not tickets:
        print("No tickets found.")
        return

    by_status: dict[Status, list] = {}
    for t in tickets:
        by_status.setdefault(t.status, []).append(t)
    for st in _DISPLAY_ORDER:
        group = by_status.get(st, [])
        if not group:
            continue
        print(f"\n{st.value.upper()} ({len(group)})")
        for t in group:
            target = (
                t.frontmatter.absorbed_into
                or t.frontmatter.duplicate_of
                or t.frontmatter.deduped_into
            )
            suffix = f" → {target}" if target else ""
            print(f"  {t.id:<30} {t.card}{suffix}")


def cmd_show(args):
    """Entry point for `./pipeline/cli.py show`."""
    t = Ticket.try_load(args.ticket_id)
    if t is None:
        print(f"Ticket not found: {args.ticket_id}")
        sys.exit(1)
    print(t.path.read_text())


def cmd_status(_args):
    """Entry point for `./pipeline/cli.py status`."""
    subprocess.run(
        ["python3", str(utils.SCRIPTS_DIR / "metrics.py")],
        cwd=str(utils.PROJECT_ROOT),
    )


def cmd_report(args):
    """Entry point for `./pipeline/cli.py report`."""
    extra = []
    if args.audits_only:
        extra.append("--audits-only")
    if args.cards_only:
        extra.append("--cards-only")
    subprocess.run(
        ["python3", str(utils.SCRIPTS_DIR / "report.py"), *extra],
        cwd=str(utils.PROJECT_ROOT),
    )
