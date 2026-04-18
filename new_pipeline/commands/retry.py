"""Retry command — recover from a terminal failure state.

Two cases handled, both following the same pattern:

- `FIX_FAILED` → mint a new ticket in `TESTED`. The test file is
  still valid; a fresh worktree is created at the old ticket's
  `tested_sha` so the fix phase starts from the post-test state
  (failed-fix commits are left behind). The new ticket's body
  inherits everything up through `## Test Run Results`, plus the
  failed ticket's `## Fix Result` wrapped as a `## Previous attempt`.
- `COULD_NOT_CONFIRM` → mint a new ticket in `NEW`. Previous tests
  were rejected (or needed engine work), so the new ticket gets no
  worktree (the test phase will create one). The new ticket's body
  inherits the audit finding + `## Tests` scenarios, plus the failed
  ticket's `## Test Run Results` wrapped as a `## Previous attempt`.

The old ticket's worktree is left in place in both cases — operators
can still inspect what the previous attempt tried. Cleanup happens
later via `close`.
"""

from __future__ import annotations

from new_pipeline import worktree
from new_pipeline.types import (
    Status,
    Ticket,
    TicketError,
    format_datetime,
)


def cmd_retry(args) -> None:
    """Entry point for `./new_pipeline/cli.py retry`."""
    ids = [i.strip() for i in args.tickets.split(",") if i.strip()]
    if not ids:
        raise ValueError("--tickets needs at least one non-empty id")
    for tid in ids:
        _retry_one(tid)


def _retry_one(tid: str) -> None:
    old = Ticket.load(tid)
    if old.status is Status.FIX_FAILED:
        new_id = _retry_from_fix_failed(old)
    elif old.status is Status.COULD_NOT_CONFIRM:
        new_id = _retry_from_could_not_confirm(old)
    else:
        raise TicketError(
            f"cannot retry a ticket in status {old.status.value!r} — "
            f"retry works on `fix_failed` and `could_not_confirm` only"
        )
    print(f"[{tid}] retried into {new_id}")


# ── Dispatch by source status ────────────────────────────────────


def _retry_from_fix_failed(old: Ticket) -> str:
    """Mint the successor for a FIX_FAILED ticket.

    Tests are still good, so a fresh worktree is created at the old
    ticket's `tested_sha` — the new branch starts from the post-test
    state, leaving failed-fix commits behind in the old worktree.
    Carries all test-phase metadata over.
    """
    stem = _stem_of(old.id)
    new_id = Ticket.allocate_id(stem)

    new_body = old.body_with_previous_attempt(
        section_heading="Fix Result",
    )

    fm = old.frontmatter
    extra = {
        "retry_of": old.id,
        # Test-phase metadata carries forward verbatim.
        "test_run_id": fm.test_run_id,
        "test_model": fm.test_model,
        "test_tokens": str(fm.test_tokens),
        "test_duration": str(fm.test_duration),
        "test_file": fm.test_file,
        "tested_sha": fm.tested_sha,
    }
    if fm.tested_at is not None:
        extra["tested_at"] = format_datetime(fm.tested_at)

    Ticket.create(
        new_id,
        status=Status.TESTED,
        card=old.frontmatter.card,
        body=new_body,
        extra=extra,
    )

    # Fresh worktree at the post-test sha; old worktree stays put.
    worktree.create_from_sha(new_id, fm.tested_sha)
    return new_id


def _retry_from_could_not_confirm(old: Ticket) -> str:
    """Mint the successor for a COULD_NOT_CONFIRM ticket.

    Previous tests were rejected (or needed engine work), so the
    new ticket starts in NEW with no worktree — the test phase will
    create one. The old worktree stays put for inspection.
    """
    stem = _stem_of(old.id)
    new_id = Ticket.allocate_id(stem)

    new_body = old.body_with_previous_attempt(
        section_heading="Test Run Results",
    )

    Ticket.create(
        new_id,
        status=Status.NEW,
        card=old.frontmatter.card,
        body=new_body,
        extra={"retry_of": old.id},
    )
    return new_id


# ── Helpers ───────────────────────────────────────────────────────


def _stem_of(ticket_id: str) -> str:
    """Strip the trailing `-NN` suffix from a ticket id to get the stem."""
    # Ticket ids look like `{stem}-{number}`, e.g. `olivia_voldaren-01`.
    # `rsplit` on the last `-` gives us everything before the number.
    stem, _, num = ticket_id.rpartition("-")
    assert num.isdigit(), f"unexpected ticket id shape: {ticket_id!r}"
    return stem
