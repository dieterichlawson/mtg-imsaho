#!/usr/bin/env python3
"""Audit coverage, per-card breakdown, and ticket backlog."""

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

_HERE = Path(__file__).resolve()
sys.path.insert(0, str(_HERE.parents[2]))

from pipeline import cli  # noqa: E402


def _load_audit_runs() -> list[dict]:

    path = cli.METRICS_DIR / "runs.jsonl"
    if not path.exists():
        return []
    runs = []
    for line in path.read_text().strip().split("\n"):
        if not line:
            continue
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        if d.get("role") == "auditor":
            runs.append(d)
    return runs


def main(argv: list[str] | None = None) -> None:
    """Print audit coverage, per-card breakdown, and ticket backlog."""
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--audits-only", action="store_true")
    p.add_argument("--cards-only", action="store_true")
    args = p.parse_args(argv)

    runs = _load_audit_runs()
    tickets = cli.list_tickets()

    # ── Audit coverage ──────────────────────────────────────────
    successful = [r for r in runs if r.get("validation_passed")]
    errored = [r for r in runs if not r.get("validation_passed")]
    cards_attempted = sorted({r["card"] for r in runs})
    cards_ok = sorted({r["card"] for r in successful})
    cards_errored_only = sorted(set(cards_attempted) - set(cards_ok))

    if not args.cards_only:
        print(f"\n{'=' * 60}\nAUDIT COVERAGE\n{'=' * 60}")
        print(f"  Total audit runs:                  {len(runs)}")
        print(f"  Successful runs:                   {len(successful)}")
        print(f"  Errored runs:                      {len(errored)}")
        print(f"  Unique cards attempted:            {len(cards_attempted)}")
        print(f"  Unique cards successfully audited: {len(cards_ok)}")
        if cards_errored_only:
            print(
                f"\n  Cards with only errored runs ({len(cards_errored_only)}):"
            )
            for c in cards_errored_only:
                reason = (
                    next(
                        (
                            r.get("rejection_reason")
                            for r in reversed(errored)
                            if r["card"] == c
                        ),
                        None,
                    )
                    or "unknown"
                )
                print(f"    {c} — {reason}")

    # ── Per-card breakdown ──────────────────────────────────────
    if not args.audits_only:
        fixed_like = {cli.STATUS_FIXED, cli.STATUS_SHIPPED}
        by_id = {t["frontmatter"].get("id"): t for t in tickets}

        def terminal_status(tid: str, seen=None) -> str:
            seen = seen or set()
            if tid in seen:
                return "(cycle)"
            seen.add(tid)
            t = by_id.get(tid)
            if not t:
                return "(unknown)"
            st = t["frontmatter"].get("status", cli.STATUS_NEW)
            if st != cli.STATUS_CLOSED:
                return st
            parent = t["frontmatter"].get("absorbed_into")
            return terminal_status(parent, seen) if parent else st

        by_card_runs = Counter(r["card"] for r in successful)
        by_card_status: dict[str, Counter] = defaultdict(Counter)
        for t in tickets:
            fm = t["frontmatter"]
            card = fm.get("card", "(unknown)")
            if card == "multiple":
                continue
            by_card_status[card][fm.get("status", cli.STATUS_NEW)] += 1

        print(f"\n{'=' * 60}\nPER-CARD BREAKDOWN\n{'=' * 60}")
        print(
            f"  {'Card':<32} {'Audits':>6} {'Tickets':>7} "
            f"{'Open':>5} {'Fixed':>5} {'Closed':>6}"
        )
        print(f"  {'-' * 32} {'-' * 6} {'-' * 7} {'-' * 5} {'-' * 5} {'-' * 6}")

        rows = []
        for card in sorted(set(cards_attempted) | set(by_card_status)):
            audits = by_card_runs.get(card, 0)
            card_tickets = [
                t for t in tickets if t["frontmatter"].get("card") == card
            ]
            open_n = fixed_n = closed_n = 0
            for t in card_tickets:
                tid = t["frontmatter"].get("id")
                st = t["frontmatter"].get("status", "new")
                if st in cli.OPEN_STATUSES:
                    open_n += 1
                elif st in fixed_like:
                    fixed_n += 1
                elif st == cli.STATUS_CLOSED:
                    if terminal_status(tid) in fixed_like:
                        fixed_n += 1
                    else:
                        closed_n += 1
            rows.append(
                (card, audits, len(card_tickets), open_n, fixed_n, closed_n)
            )

        rows.sort(key=lambda r: (-r[2], r[0]))
        for card, a, tot, op, fx, cl in rows:
            print(f"  {card:<32} {a:>6} {tot:>7} {op:>5} {fx:>5} {cl:>6}")

        clean = sum(1 for r in rows if r[1] > 0 and r[2] == 0)
        print(
            f"\n  {len(rows)} cards total — {clean} clean (Audits>0, Tickets=0)"
        )

    # ── Ticket backlog ──────────────────────────────────────────
    if not args.cards_only and not args.audits_only:
        print(f"\n{'=' * 60}\nTICKET BACKLOG\n{'=' * 60}")
        status_counts = Counter(
            t["frontmatter"].get("status", cli.STATUS_NEW) for t in tickets
        )
        primary = [
            cli.STATUS_NEW,
            cli.STATUS_TESTED,
            cli.STATUS_FIXED,
            cli.STATUS_FIX_FAILED,
            cli.STATUS_FALSE_POSITIVE,
            cli.STATUS_SHIPPED,
            cli.STATUS_CLOSED,
        ]
        other = sorted(s for s in status_counts if s not in primary)
        for s in primary + other:
            if status_counts.get(s):
                print(f"  {s:<14} {status_counts[s]:>4}")
        merged_parents = [
            t for t in tickets if t["frontmatter"].get("card") == "multiple"
        ]
        print(f"\n  Merged (parent) tickets: {len(merged_parents)}")
    print()


if __name__ == "__main__":
    main()
