#!/usr/bin/env python3
"""Pipeline CLI — entry point for argparse + command dispatch.

See `pipeline/state.py` for the ticket state machine and each
`pipeline/commands/<name>.py` for the implementation of a subcommand.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# Allow `./pipeline/cli.py` to run as a script without `python -m`.
_THIS = Path(__file__).resolve()
if str(_THIS.parents[1]) not in sys.path:
    sys.path.insert(0, str(_THIS.parents[1]))

from pipeline import utils
from pipeline.agent import DEFAULT_EFFORT, DEFAULT_MODEL
from pipeline.commands import (
    audit,
    close,
    consolidate,
    dedup,
    fix,
    merge,
    retry,
    show,
    test,
)
from pipeline.state import Status


def _add_agent_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("--model", default=DEFAULT_MODEL)
    p.add_argument("--effort", default=DEFAULT_EFFORT)


def main() -> None:
    """Parse command-line args and dispatch to the matching `cmd_*`."""
    parser = argparse.ArgumentParser(
        description="Bug-pipeline CLI — find, test, fix, merge, dedup."
    )
    sub = parser.add_subparsers(dest="command", required=True)

    def P(name, **kw):

        return sub.add_parser(name, **kw)

    p = P("audit", help="Audit cards for bugs")
    _add_agent_args(p)
    p.add_argument(
        "--cards",
        required=True,
        help="Card names separated by ',' (or ';' if a name has a comma)",
    )
    p.add_argument("--parallelism", type=int, default=1)

    p = P("test", help="Write tests for new tickets")
    _add_agent_args(p)
    p.add_argument("--tickets", help="Comma-separated ticket IDs")
    p.add_argument("--parallelism", type=int, default=1)

    p = P("fix", help="Fix a tested ticket")
    _add_agent_args(p)
    p.add_argument("--ticket")

    p = P("tickets", help="List tickets")
    p.add_argument("--status")
    p.add_argument("--card")

    p = P("show", help="Display a ticket")
    p.add_argument("ticket_id")

    p = P("merge", help="Merge fixed ticket(s) to HEAD")
    p.add_argument("ticket_id", help="Ticket ID, comma-separated IDs, or 'all'")

    p = P("retry", help="Rewind a ticket to an earlier phase")
    p.add_argument("ticket_id")
    p.add_argument("--to", choices=[Status.NEW.value, Status.TESTED.value])
    p.add_argument(
        "--force",
        action="store_true",
        help=f"Allow retry on {Status.FALSE_POSITIVE.value}",
    )

    p = P("close", help="Close a ticket as abandoned")
    p.add_argument("ticket_id")
    p.add_argument("--note")

    P("status", help="Show metrics dashboard")

    p = P("report", help="Coverage + backlog report")
    p.add_argument("--audits-only", action="store_true")
    p.add_argument("--cards-only", action="store_true")

    p = P("consolidate", help="Ingest a consolidation staging file → merged-*")
    p.add_argument("--input", required=True)
    p.add_argument("--keep-input", action="store_true")
    p.add_argument("--dry-run", action="store_true")

    p = P("dedup", help="Spawn a dedup agent on a seed set of tickets")
    p.add_argument(
        "--tickets", required=True, help="Comma-separated seed ticket IDs"
    )
    p.add_argument("--keep-input", action="store_true")
    _add_agent_args(p)

    args = parser.parse_args()
    utils.TICKETS_DIR.mkdir(parents=True, exist_ok=True)
    utils.STAGING_DIR.mkdir(parents=True, exist_ok=True)
    utils.LOGS_DIR.mkdir(parents=True, exist_ok=True)

    {
        "audit": audit.cmd_audit,
        "test": test.cmd_test,
        "fix": fix.cmd_fix,
        "merge": merge.cmd_merge,
        "retry": retry.cmd_retry,
        "close": close.cmd_close,
        "tickets": show.cmd_tickets,
        "show": show.cmd_show,
        "status": show.cmd_status,
        "consolidate": consolidate.cmd_consolidate,
        "dedup": dedup.cmd_dedup,
        "report": show.cmd_report,
    }[args.command](args)


if __name__ == "__main__":
    main()
