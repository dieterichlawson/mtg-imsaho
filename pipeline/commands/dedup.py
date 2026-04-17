"""Dedup — spawn an agent on a seed set of open tickets, ingest each.

consolidation it produces via cmd_consolidate.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from pipeline import ticket, utils
from pipeline.agent import MAX_ATTEMPTS, build_prompt, run_agent
from pipeline.commands.consolidate import cmd_consolidate
from pipeline.staging import ConsolidationProposal, StagingError
from pipeline.state import Status
from pipeline.utils import today

# Any of these statuses means the ticket is already accounted for in a
# parent's fix or shipped commit — it's not a valid dedup candidate.
_CLOSED_FOR_DEDUP = {Status.CLOSED, Status.FIXED, Status.SHIPPED}


def cmd_dedup(args):
    """Entry point for `./pipeline/cli.py dedup`."""
    ids = [t.strip() for t in args.tickets.split(",") if t.strip()]
    if not ids:
        print("ERROR: dedup requires at least one candidate ticket id")
        sys.exit(1)

    candidates = _load_candidates(ids)
    section = _format_tickets_section(candidates)
    builder = build_prompt(
        "dedup", num_tickets=len(ids), tickets_section=section
    )

    if args.dry_run:
        print(
            f"[dry-run] Would spawn dedup agent for {len(ids)} candidate "
            f"ticket(s) (model={args.model})"
        )
        return

    preexisting = {f.name for f in _existing_consolidations()}
    retry_note = ""
    last_error: str | None = None

    for attempt in range(1, MAX_ATTEMPTS + 1):
        print(f"\nSpawning dedup agent (attempt {attempt}/{MAX_ATTEMPTS})...")
        result = run_agent(
            builder(retry_note, attempt),
            args.model,
            args.effort,
            log_path=utils.LOGS_DIR / f"{today()}-dedup-attempt{attempt}.log",
            progress_prefix="  [dedup] ",
        )

        if result.get("is_error"):
            last_error = result.get("error_message") or "unknown agent error"
            print(f"  Agent error: {last_error} ({result['duration']}s)")
            retry_note = _retry_note_agent_error(attempt, last_error)
            continue

        new_files = [
            f for f in _existing_consolidations() if f.name not in preexisting
        ]
        if not new_files:
            print(
                f"  Agent produced no consolidation files — nothing to merge "
                f"({result['duration']}s, {result['tokens']} tok)"
            )
            return

        errs = _validate_consolidations(new_files)
        if errs:
            last_error = "; ".join(f"{f.name}: {m}" for f, m in errs)
            retry_note = _retry_note_parse_errors(attempt, errs)
            continue

        _ingest_all(new_files, args.keep_input)
        return

    print(
        f"\nERROR: dedup failed after {MAX_ATTEMPTS} attempt(s). "
        f"Last error: {last_error}"
    )
    sys.exit(1)


def _load_candidates(ids: list[str]) -> list[tuple[str, str]]:
    out = []
    for tid in ids:
        t = ticket.get_ticket_if_exists(tid)
        if t is None:
            print(f"ERROR: ticket not found: {tid}")
            sys.exit(1)
        if t.status in _CLOSED_FOR_DEDUP:
            print(f"ERROR: {tid} already closed (status={t.status.value})")
            sys.exit(1)
        out.append((tid, t.path.read_text()))
    return out


def _format_tickets_section(candidates: list[tuple[str, str]]) -> str:
    return "\n\n---\n\n".join(
        f"## Candidate ticket: {tid}\n\n{body}" for tid, body in candidates
    )


def _existing_consolidations() -> list[Path]:
    return sorted(utils.STAGING_DIR.glob("consolidation-*.json"))


def _validate_consolidations(files: list[Path]) -> list[tuple[Path, str]]:
    errs = []
    for f in files:
        try:
            ConsolidationProposal.load(f)
        except StagingError as e:
            errs.append((f, str(e)))
    if errs:
        print(f"  Parse errors in {len(errs)} file(s)")
        for f, m in errs:
            print(f"    {f.name}: {m}")
    return errs


def _ingest_all(files: list[Path], keep_input: bool) -> None:
    print(f"  Agent produced {len(files)} valid staging file(s)")
    for f in files:
        print(f"\n── consolidating {f.name} ──")
        cmd_consolidate(
            argparse.Namespace(
                input=str(f), keep_input=keep_input, dry_run=False
            )
        )


def _retry_note_agent_error(attempt: int, error: str) -> str:
    return (
        f"\n\n## Retry note (attempt {attempt} failed)\n"
        f"Previous attempt errored: {error}\n"
    )


def _retry_note_parse_errors(
    attempt: int, errors: list[tuple[Path, str]]
) -> str:
    detail = "\n".join(f"- {f.name}: {m}" for f, m in errors)
    return (
        f"\n\n## Retry note (attempt {attempt} failed)\n"
        f"Previous attempt produced {len(errors)} unparseable "
        f"staging file(s):\n{detail}\nEdit them in place to fix.\n"
    )
