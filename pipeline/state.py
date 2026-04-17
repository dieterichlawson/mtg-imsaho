"""The ticket state machine — statuses and the transitions between them.

Every status change in the pipeline goes through one of the `after_*`
functions below. They are pure: status in, status out. Side effects
(writing the ticket file, archiving, logging) live in ticket.py and
the command modules.
"""
from __future__ import annotations

from enum import Enum


class Status(str, Enum):
    """A ticket's lifecycle state. Mixes with `str` so comparisons with
    raw frontmatter strings still work transparently.
    """

    NEW            = "new"
    TESTED         = "tested"
    FIXED          = "fixed"
    SHIPPED        = "shipped"
    FIX_FAILED     = "fix_failed"
    FALSE_POSITIVE = "false_positive"
    CLOSED         = "closed"

    @property
    def is_terminal(self) -> bool:
        """True if no further transitions are allowed (shipped/closed/FP)."""
        return self in _TERMINAL

    @property
    def is_open(self) -> bool:
        """True if the ticket is still somewhere in the active pipeline."""
        return not self.is_terminal

    @property
    def is_absorbable(self) -> bool:
        """Eligible to be folded into a merged-* parent via consolidate/dedup."""
        return self in _ABSORBABLE


_TERMINAL   = frozenset({Status.SHIPPED, Status.FALSE_POSITIVE, Status.CLOSED})
_ABSORBABLE = frozenset({Status.NEW, Status.TESTED})


class CloseReason(str, Enum):
    """Why a ticket ended up in `closed`."""

    ABSORBED  = "absorbed"   # folded into a merged-* parent
    ABANDONED = "abandoned"  # human gave up


class TestOutcome(str, Enum):
    """Aggregated verdict of the test-writer on a ticket."""

    CONFIRMED    = "confirmed"      # at least one test compiles + fails
    NEEDS_ENGINE = "needs_engine"   # one or more blocked on engine surface
    NO_REAL_BUG  = "no_real_bug"    # nothing confirmed, no blockers


class FixOutcome(str, Enum):
    """Aggregated verdict of the fixer on a ticket."""

    SUCCEEDED = "succeeded"   # validate_fix passed
    FAILED    = "failed"      # fixer gave up or validation rejected


# ─── Transitions ─────────────────────────────────────────────────────

def after_audit() -> Status:
    """A freshly-created audit ticket starts here."""
    return Status.NEW


def after_test(current: Status, outcome: TestOutcome) -> Status:
    """Route a ticket after the test-writer runs."""
    _require(current, {Status.NEW}, f"test phase on {current}")
    if outcome is TestOutcome.CONFIRMED:
        return Status.TESTED
    if outcome is TestOutcome.NEEDS_ENGINE:
        # Re-enter `new` so test-writer runs again with allow_engine_edits set.
        return Status.NEW
    return Status.FALSE_POSITIVE


def after_fix(current: Status, outcome: FixOutcome) -> Status:
    """Route a ticket after the fixer runs."""
    _require(current, {Status.TESTED}, f"fix phase on {current}")
    return Status.FIXED if outcome is FixOutcome.SUCCEEDED else Status.FIX_FAILED


def after_merge(current: Status) -> Status:
    """Route a ticket after its branch is merged into master."""
    _require(current, {Status.FIXED}, f"merge on {current}")
    return Status.SHIPPED


def after_absorb(current: Status) -> Status:
    """Consolidate/dedup absorbs a ticket into a merged-* parent."""
    if not current.is_absorbable:
        raise ValueError(
            f"cannot absorb a ticket in status {current!r}; "
            f"only {sorted(s.value for s in _ABSORBABLE)} are absorbable")
    return Status.CLOSED


def after_abandon(current: Status) -> Status:
    """Human gives up and closes the ticket manually."""
    if current.is_terminal:
        raise ValueError(
            f"cannot abandon a ticket already in terminal status {current!r}")
    return Status.CLOSED


def after_retry(current: Status, target: Status, force: bool = False) -> Status:
    """Rewind a ticket to an earlier phase.

    `target` is chosen by the caller (user can pass --to). Defaults live
    in the retry command, not here; this function validates legality.
    """
    if current is Status.FALSE_POSITIVE and not force:
        raise ValueError(f"{current} requires --force to retry")
    if current in {Status.SHIPPED, Status.CLOSED}:
        raise ValueError(f"{current} is terminal; cannot retry")
    if target not in {Status.NEW, Status.TESTED}:
        raise ValueError(
            f"retry target must be {Status.NEW} or {Status.TESTED}, got {target}")
    return target


# ─── Internals ───────────────────────────────────────────────────────

def _require(current: Status, allowed: set, what: str) -> None:
    if current not in allowed:
        raise ValueError(
            f"{what}: expected one of {sorted(s.value for s in allowed)}, "
            f"got {current!r}")
