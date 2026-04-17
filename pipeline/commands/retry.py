"""Rewind a ticket to an earlier phase (in place) without spawning an agent."""

from __future__ import annotations

import re
import sys

from pipeline import worktree
from pipeline.models import Ticket
from pipeline.state import Status, retry_target

# Frontmatter fields that belong to each phase. Cleared when a retry
# rewinds past that phase so stale metadata doesn't carry forward.
_FIX_KEYS = (
    "fix_run_id",
    "fix_model",
    "fix_tokens",
    "fix_duration",
    "fixed_at",
    "fixed_sha",
    "fix_failed_at",
)
_TEST_KEYS = (
    "test_run_id",
    "test_model",
    "test_tokens",
    "test_duration",
    "test_file",
    "tests_confirmed",
    "tests_total",
    "tested_at",
    "tested_sha",
    "worktree",
    "engine_block_at",
    "false_positive_at",
    "allow_engine_edits",
)


def cmd_retry(args):
    """Entry point for `./pipeline/cli.py retry`."""
    t = Ticket.load(args.ticket_id)
    previous = t.status
    target = Status(args.to) if args.to else _default_target(previous)

    try:
        t.status = retry_target(previous, target, force=args.force)
    except ValueError as e:
        print(f"ERROR: {e}")
        sys.exit(1)

    sha = t.tested_sha if target is Status.TESTED else "master"
    if target is Status.TESTED and not sha:
        print(
            f"ERROR: {t.id} has no tested_sha; cannot rewind to "
            f"{Status.TESTED.value}. Use --to {Status.NEW.value} instead."
        )
        sys.exit(1)

    print(
        f"[{t.id}] Retry: {previous.value} → {target.value}  (reset to {sha})"
    )
    _archive_prior_attempt(t)

    if target is Status.NEW:
        worktree.remove(t.id)
        t.reset_test_implementations()
        _clear(t, _FIX_KEYS + _TEST_KEYS)
    else:
        worktree.reset_to(t.id, sha)
        _clear(t, _FIX_KEYS)
    t.save()

    cmd = "test" if target is Status.NEW else "fix"
    flag = "--tickets" if target is Status.NEW else "--ticket"
    print(
        f"[{t.id}] Ready. "
        f"Run `./pipeline/cli.py {cmd} {flag} {t.id}` to continue."
    )


def _default_target(current: Status) -> Status:
    """`fix_failed` rewinds to tested (keep the tests); anything else → new."""
    return Status.TESTED if current is Status.FIX_FAILED else Status.NEW


def _clear(t, keys: tuple[str, ...]) -> None:
    t.frontmatter.clear(*keys)


def _archive_prior_attempt(t) -> None:
    """Fold `## Fix Result` / `## Test Run Results` / `## Engine Change.

    Needed` sections into a numbered `## Attempt N` archive block so retry
    history is preserved in the ticket body.
    """
    body = t.body
    archivable = (
        "## Fix Result",
        "## Test Run Results",
        "## Engine Change Needed",
    )
    nums = [
        int(m.group(1))
        for m in re.finditer(r"^##\s+Attempt\s+(\d+)\b", body, re.MULTILINE)
    ]
    next_n = (max(nums) + 1) if nums else 1

    sections: list[str] = []
    keep: list[str] = []
    current: list[str] | None = None
    for line in body.split("\n"):
        if line.startswith("## ") and any(
            line.startswith(h) for h in archivable
        ):
            if current is not None:
                sections.append("\n".join(current).rstrip())
            current = [line]
        elif line.startswith("## ") and current is not None:
            sections.append("\n".join(current).rstrip())
            current = None
            keep.append(line)
        elif current is not None:
            current.append(line)
        else:
            keep.append(line)
    if current is not None:
        sections.append("\n".join(current).rstrip())
    if not sections:
        return
    t.body = (
        "\n".join(keep).rstrip()
        + "\n\n"
        + f"## Attempt {next_n}\n\n"
        + "\n\n".join(sections).rstrip()
        + "\n"
    )
