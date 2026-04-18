"""Retry command — recover from a terminal failure state.

Two cases handled, both following the same pattern:

- `FIX_FAILED` → mint a new ticket in `TESTED`. The test file is
  still valid; the worktree is renamed over to the new ticket so
  the fix phase can run directly. The new ticket's body inherits
  everything up through `## Test Run Results`, plus the failed
  ticket's `## Fix Result` wrapped as a `## Previous attempt`.
- `COULD_NOT_CONFIRM` → mint a new ticket in `NEW`. Previous tests
  were rejected (or needed engine work), so the worktree is
  discarded. The new ticket's body inherits the audit finding +
  `## Tests` scenarios, plus the failed ticket's
  `## Test Run Results` wrapped as a `## Previous attempt`.

In both cases the old ticket stays archived as an immutable record;
the new ticket carries a `retry_of` frontmatter pointer to it.
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

    Tests are still good, so the worktree is renamed to the new
    ticket. Carries all test-phase metadata over.
    """
    stem = _stem_of(old.id)
    new_id = Ticket.allocate_id(stem)

    new_body = _body_with_previous_attempt(
        old.body, section_heading="Fix Result", old_id=old.id,
    )

    # Mint the new ticket first — takes the worktree-less slot in
    # the active tickets dir. The worktree rename lands after.
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

    # Rename the worktree/branch to the new id. Commits preserved,
    # tested_sha still points at the same commit.
    worktree.rename(old.id, new_id)
    return new_id


def _retry_from_could_not_confirm(old: Ticket) -> str:
    """Mint the successor for a COULD_NOT_CONFIRM ticket.

    Previous tests were rejected (or needed engine work), so the
    worktree is torn down and the new ticket starts in NEW — the
    test phase will run fresh.
    """
    stem = _stem_of(old.id)
    new_id = Ticket.allocate_id(stem)

    new_body = _body_with_previous_attempt(
        old.body, section_heading="Test Run Results", old_id=old.id,
    )

    Ticket.create(
        new_id,
        status=Status.NEW,
        card=old.frontmatter.card,
        body=new_body,
        extra={"retry_of": old.id},
    )
    # The old worktree holds partial/rejected test work — drop it so
    # the next test run starts from scratch.
    worktree.remove(old.id)
    return new_id


# ── Helpers ───────────────────────────────────────────────────────


def _stem_of(ticket_id: str) -> str:
    """Strip the trailing `-NN` suffix from a ticket id to get the stem."""
    # Ticket ids look like `{stem}-{number}`, e.g. `olivia_voldaren-01`.
    # `rsplit` on the last `-` gives us everything before the number.
    stem, _, num = ticket_id.rpartition("-")
    assert num.isdigit(), f"unexpected ticket id shape: {ticket_id!r}"
    return stem


def _body_with_previous_attempt(
    body: str, *, section_heading: str, old_id: str,
) -> str:
    """Return `body` with its `## {section_heading}` section rewrapped.

    The named section is extracted, its contents (without the
    original `##` heading line) become a `## Previous attempt
    ({old_id})` section appended to the end. If the named section
    isn't found, the original body is returned with a bare
    `## Previous attempt ({old_id})` note pointing at the old id.
    """
    before, section, after = _split_off_section(body, section_heading)
    inner = _strip_leading_heading(section) if section else (
        f"See archived ticket `{old_id}` for details."
    )
    remainder = (before.rstrip() + ("\n\n" + after.lstrip() if after else ""))
    prev = f"## Previous attempt ({old_id})\n\n{inner.rstrip()}"
    return remainder.rstrip() + "\n\n" + prev + "\n"


def _split_off_section(
    body: str, heading: str,
) -> tuple[str, str, str]:
    """Split `body` around a `## {heading}` section.

    Returns (before, section, after). `section` includes its own
    heading line and runs until the next `## ` heading or end of
    body. If the heading isn't found, returns `(body, "", "")`.
    """
    lines = body.split("\n")
    start: int | None = None
    for i, line in enumerate(lines):
        if line.strip() == f"## {heading}":
            start = i
            break
    if start is None:
        return body, "", ""
    end = len(lines)
    for j in range(start + 1, len(lines)):
        if lines[j].startswith("## "):
            end = j
            break
    before = "\n".join(lines[:start]).rstrip()
    section = "\n".join(lines[start:end]).rstrip()
    after = "\n".join(lines[end:]).strip()
    return before, section, after


def _strip_leading_heading(section: str) -> str:
    """Drop the `## ...` line at the top of `section`, return the rest."""
    _, _, rest = section.partition("\n")
    return rest.lstrip()
