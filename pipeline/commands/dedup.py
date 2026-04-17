"""Dedup — spawn an agent on a seed set of open tickets, ingest each.

consolidation it produces via cmd_consolidate.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from pipeline import utils
from pipeline.agent import build_prompt, run_agent_loop
from pipeline.commands.consolidate import cmd_consolidate
from pipeline.models import ConsolidationProposal, StagingError, Ticket
from pipeline.state import Status
from pipeline.utils import today

# Statuses that mean the ticket is already accounted for in a parent's
# fix or shipped commit — not a valid dedup candidate.
_CLOSED_FOR_DEDUP = {Status.CLOSED, Status.FIXED, Status.SHIPPED}


def cmd_dedup(args):
    """Entry point for `./pipeline/cli.py dedup`."""
    ids = [t.strip() for t in args.tickets.split(",") if t.strip()]
    if not ids:
        print("ERROR: dedup requires at least one candidate ticket id")
        sys.exit(1)

    candidates = _load_candidates(ids)
    tickets_section = "\n\n---\n\n".join(
        f"## Candidate ticket: {tid}\n\n{body}" for tid, body in candidates
    )
    builder = build_prompt(
        "dedup", num_tickets=len(ids), tickets_section=tickets_section
    )

    preexisting = {f.name for f in _existing_consolidations()}
    proposals, result = run_agent_loop(
        build_prompt=builder,
        cwd=utils.PROJECT_ROOT,
        load_result=_consolidation_loader(preexisting),
        log_prefix=f"{today()}-dedup",
        progress_prefix="  [dedup] ",
        model=args.model,
        effort=args.effort,
    )

    if result.get("is_error"):
        print(f"\nERROR: dedup failed. {result.get('error_message')}")
        sys.exit(1)
    if not proposals:
        print(
            f"  Agent produced no consolidation files — nothing to merge "
            f"({result['duration']}s, {result['tokens']} tok)"
        )
        return

    print(f"  Agent produced {len(proposals)} valid staging file(s)")
    for f in proposals:
        print(f"\n── consolidating {f.name} ──")
        cmd_consolidate(
            argparse.Namespace(
                input=str(f), keep_input=args.keep_input, dry_run=False
            )
        )


def _load_candidates(ids: list[str]) -> list[tuple[str, str]]:
    out = []
    for tid in ids:
        t = Ticket.try_load(tid)
        if t is None:
            print(f"ERROR: ticket not found: {tid}")
            sys.exit(1)
        if t.status in _CLOSED_FOR_DEDUP:
            print(f"ERROR: {tid} already closed (status={t.status.value})")
            sys.exit(1)
        out.append((tid, t.path.read_text()))
    return out


def _existing_consolidations() -> list[Path]:
    return sorted(utils.STAGING_DIR.glob("consolidation-*.json"))


def _consolidation_loader(preexisting: set[str]):
    """Build a `load_result` that scans staging for new consolidation files.

    Returns (list_of_new_paths, None) on success. If any new file fails
    to parse, returns (None, retry_note) listing each file's error.
    """
    def _load(result: dict, _attempt: int):
        if result.get("is_error"):
            err_msg = result.get("error_message") or "unknown"
            return None, f"Previous attempt errored: {err_msg}"
        new_files = [
            f for f in _existing_consolidations()
            if f.name not in preexisting
        ]
        errs: list[tuple[Path, str]] = []
        for f in new_files:
            try:
                ConsolidationProposal.load(f)
            except StagingError as e:
                errs.append((f, str(e)))
        if errs:
            detail = "\n".join(f"- {f.name}: {m}" for f, m in errs)
            return None, (
                f"Previous attempt produced {len(errs)} unparseable "
                f"staging file(s):\n{detail}\nEdit them in place to fix."
            )
        return new_files, None
    return _load
