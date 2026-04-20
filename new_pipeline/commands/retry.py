"""Retry command — recover from a terminal-but-retryable state.

Three retryable statuses, all following the same pattern:

- `FIX_FAILED` → mint a new ticket in `TESTED`. The test file is
  still valid; a fresh worktree is created at the old ticket's
  `tested_sha` so the fix phase starts from the post-test state
  (failed-fix commits are left behind). The new ticket's body
  inherits everything up through `## Test Run Results`, plus the
  failed ticket's `## Fix Result` wrapped as a `## Previous attempt`.
- `ENGINE_BLOCKED` → mint a new ticket in `NEW`. The previous tests
  all needed engine surface; the fresh ticket gets no worktree (the
  test phase will create one) and the test-writer will run with the
  `test_writer_engine` sandbox so it can add the missing surface.
- `MIXED` → mint a new ticket in `NEW`. Same mechanics as
  ENGINE_BLOCKED — operator is expected to trim or edit the
  scenarios in the new ticket before re-running `test`.

The old ticket's worktree is left in place for inspection. Cleanup
happens later via `close`.
"""

from __future__ import annotations

from new_pipeline import utils, worktree
from new_pipeline.types import (
    Status,
    Ticket,
    TicketError,
    format_datetime,
)


def cmd_retry(args) -> None:
    """Entry point for `./new_pipeline/cli.py retry`."""
    ids = utils.split_csv(args.tickets)
    if not ids:
        raise ValueError("--tickets needs at least one non-empty id")
    for tid in ids:
        _retry_one(tid)


_RETRY_FROM_TEST_PHASE = {Status.ENGINE_BLOCKED, Status.MIXED}


def _retry_one(tid: str) -> None:
    old = Ticket.load(tid)
    if old.status is Status.FIX_FAILED:
        new_id = _retry_from_fix_failed(old)
    elif old.status in _RETRY_FROM_TEST_PHASE:
        new_id = _retry_from_test_phase(old)
    else:
        raise TicketError(
            f"cannot retry a ticket in status {old.status.value!r} — "
            f"retry works on `fix_failed`, `engine_blocked`, and `mixed` only"
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


def _retry_from_test_phase(old: Ticket) -> str:
    """Mint the successor for a test-phase failure (ENGINE_BLOCKED or MIXED).

    Previous tests couldn't confirm the full scenario set, so the
    new ticket starts in NEW with no worktree — the test phase will
    create one. The `retry_of` pointer tells the test command to use
    the `test_writer_engine` sandbox (engine-edit access), useful for
    ENGINE_BLOCKED and harmless for MIXED.
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
