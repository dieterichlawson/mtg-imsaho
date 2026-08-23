#!/usr/bin/env python3
"""Set the status (and phase metadata) on new_pipeline tickets.

For hand-worked fixes that land outside a `new_pipeline` agent run: the
pipeline's own commands write frontmatter through `new_pipeline.types`,
but a human/agent fixing a whole root-cause cluster in one commit needs
to stamp the same fields across many tickets at once.

    scripts/mark_ticket_status.py fixed --sha <sha> \
        --test-file mtg-engine/tests/foo.rs --note "..." ticket-01 ticket-02

Only the keys this script sets are rewritten; every other line of the
frontmatter is preserved verbatim.
"""

from __future__ import annotations

import argparse
import sys
from datetime import datetime, timezone
from pathlib import Path

TICKETS = Path(__file__).resolve().parent.parent / "new_pipeline" / "tickets"


def set_keys(path: Path, updates: dict[str, str]) -> None:
    lines = path.read_text().split("\n")
    if lines[0] != "---":
        raise SystemExit(f"{path}: no frontmatter")
    end = lines.index("---", 1)
    head, body = lines[1:end], lines[end:]

    remaining = dict(updates)
    out = []
    for line in head:
        key = line.split(":", 1)[0].strip()
        if key in remaining:
            out.append(f"{key}: {remaining.pop(key)}")
        else:
            out.append(line)
    out.extend(f"{k}: {v}" for k, v in remaining.items())
    path.write_text("\n".join(["---", *out, *body]))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("status", help="new status, e.g. fixed / closed")
    ap.add_argument("tickets", nargs="+", help="ticket ids (no .md suffix)")
    ap.add_argument("--sha", default="", help="commit sha carrying the fix")
    ap.add_argument("--test-file", default="", help="regression test path")
    ap.add_argument("--note", default="", help="one-line explanation")
    args = ap.parse_args()

    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    updates = {"status": args.status}
    if args.sha:
        updates["fixed_sha"] = args.sha
        updates["fixed_at"] = now
    if args.test_file:
        updates["test_file"] = args.test_file
    if args.note:
        updates["fix_note"] = args.note

    missing = [t for t in args.tickets if not (TICKETS / f"{t}.md").is_file()]
    if missing:
        print(f"no such ticket(s): {', '.join(missing)}", file=sys.stderr)
        return 1

    for t in args.tickets:
        set_keys(TICKETS / f"{t}.md", dict(updates))
        print(f"{t}: {args.status}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
