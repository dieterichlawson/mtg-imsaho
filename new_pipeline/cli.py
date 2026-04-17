#!/usr/bin/env python3
"""Entry point for the new_pipeline CLI.

Usage:
    ./new_pipeline/cli.py tickets [--status X] [--card Y]
    ./new_pipeline/cli.py show <ticket-id>
    ./new_pipeline/cli.py close <ticket-id> [--note "..."]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# Make `new_pipeline.*` importable when this script is invoked directly.
if __name__ == "__main__" and __package__ is None:
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from new_pipeline.commands import close, show  # noqa: E402


def main() -> None:
    """Parse command-line args and dispatch to the matching `cmd_*`."""
    parser = argparse.ArgumentParser(description="Bug-ticket CLI")
    sub = parser.add_subparsers(dest="command", required=True)

    p_tickets = sub.add_parser("tickets", help="List tickets")
    p_tickets.add_argument("--status")
    p_tickets.add_argument("--card")

    p_show = sub.add_parser("show", help="Print one ticket file")
    p_show.add_argument("ticket_id")

    p_close = sub.add_parser("close", help="Close a ticket as abandoned")
    p_close.add_argument("ticket_id")
    p_close.add_argument("--note", help="Optional note recorded in frontmatter")

    args = parser.parse_args()
    dispatch = {
        "tickets": show.cmd_tickets,
        "show": show.cmd_show,
        "close": close.cmd_close,
    }
    dispatch[args.command](args)


if __name__ == "__main__":
    main()
