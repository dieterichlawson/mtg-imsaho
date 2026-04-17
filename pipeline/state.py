"""The ticket state machine — statuses, outcomes, and one transition function.

Every status change in the pipeline goes through `next_status`. It is
pure: current status + outcome → new status. Side effects (writing the
ticket file, archiving, metrics) live in ticket.py and the command
modules.
"""

from __future__ import annotations

from enum import Enum


class Status(str, Enum):
    """A ticket's lifecycle state.

    Mixes with `str` so comparisons against raw frontmatter strings
    still work transparently.
    """

    NEW = "new"
    TESTED = "tested"
    FIXED = "fixed"
    SHIPPED = "shipped"
    FIX_FAILED = "fix_failed"
    FALSE_POSITIVE = "false_positive"
    CLOSED = "closed"

    @property
    def is_terminal(self) -> bool:
        """True if no further transitions are allowed (shipped/closed/FP)."""
        return self in {Status.SHIPPED, Status.FALSE_POSITIVE, Status.CLOSED}

    @property
    def is_open(self) -> bool:
        """True if the ticket is still somewhere in the active pipeline."""
        return not self.is_terminal

    @property
    def is_absorbable(self) -> bool:
        """Eligible to be folded into a merged-* parent via consolidate.

        `fixed` is included: consolidate drops the fix commits and the
        parent re-enters the fix phase. `fix_failed` is not — its
        post-mortem commits are the only artifact of a failed run and
        silently losing them on absorption would hide real information.
        """
        return self in {Status.NEW, Status.TESTED, Status.FIXED}


class CloseReason(str, Enum):
    """Why a ticket ended up in `closed`."""

    ABSORBED = "absorbed"  # folded into a merged-* parent
    ABANDONED = "abandoned"  # human gave up


class TestOutcome(str, Enum):
    """Aggregated verdict of the test-writer on a ticket."""

    CONFIRMED = "confirmed"  # at least one test compiles + fails
    NEEDS_ENGINE = "needs_engine"  # one or more blocked on engine surface
    NO_REAL_BUG = "no_real_bug"  # nothing confirmed, no blockers


class FixOutcome(str, Enum):
    """Aggregated verdict of the fixer on a ticket."""

    SUCCEEDED = "succeeded"  # validate_fix passed
    FAILED = "failed"  # fixer gave up or validation rejected


class LifecycleEvent(str, Enum):
    """Non-stage events that still change a ticket's status."""

    MERGED = "merged"  # branch merged into master
    ABSORBED = "absorbed"  # folded into a merged-* parent
    ABANDONED = "abandoned"  # human gave up


# Every outcome the state machine knows how to route. Callers pass one
# of these to `next_status` alongside the current status.
Outcome = TestOutcome | FixOutcome | LifecycleEvent


# (from_status, outcome) → to_status. Missing pairs are illegal.
_TRANSITIONS: dict[tuple[Status, Outcome], Status] = {
    (Status.NEW, TestOutcome.CONFIRMED): Status.TESTED,
    # NEEDS_ENGINE re-enters `new` so the next test run sees
    # allow_engine_edits=true and may modify engine source.
    (Status.NEW, TestOutcome.NEEDS_ENGINE): Status.NEW,
    (Status.NEW, TestOutcome.NO_REAL_BUG): Status.FALSE_POSITIVE,
    (Status.TESTED, FixOutcome.SUCCEEDED): Status.FIXED,
    (Status.TESTED, FixOutcome.FAILED): Status.FIX_FAILED,
    (Status.FIXED, LifecycleEvent.MERGED): Status.SHIPPED,
}


class IllegalTransitionError(ValueError):
    """Raised when no valid transition exists for (current, outcome)."""


def next_status(current: Status, outcome: Outcome) -> Status:
    """Return the status a ticket should land in after `outcome`.

    Raises IllegalTransitionError if the pair isn't in the state machine.
    Absorbed/abandoned apply from any open/absorbable state and are
    handled explicitly — the rest are a pure table lookup.
    """
    if outcome is LifecycleEvent.ABSORBED:
        if not current.is_absorbable:
            raise IllegalTransitionError(
                f"cannot absorb a ticket in status {current.value!r}; "
                f"only 'new' and 'tested' are absorbable"
            )
        return Status.CLOSED
    if outcome is LifecycleEvent.ABANDONED:
        if current.is_terminal:
            raise IllegalTransitionError(
                f"cannot abandon a ticket already terminal ({current.value!r})"
            )
        return Status.CLOSED
    try:
        return _TRANSITIONS[(current, outcome)]
    except KeyError:
        raise IllegalTransitionError(
            f"no transition: status={current.value!r}, "
            f"outcome={outcome.value!r}"
        ) from None


def retry_target(
    current: Status, target: Status, *, force: bool = False
) -> Status:
    """Validate a retry rewind request.

    Unlike `next_status`, the caller picks the target explicitly (via
    `--to`). We just enforce the rules around terminality and force.
    """
    if current is Status.FALSE_POSITIVE and not force:
        raise IllegalTransitionError(
            f"{current.value!r} requires --force to retry"
        )
    if current in {Status.SHIPPED, Status.CLOSED}:
        raise IllegalTransitionError(
            f"{current.value!r} is terminal; cannot retry"
        )
    if target not in {Status.NEW, Status.TESTED}:
        raise IllegalTransitionError(
            f"retry target must be 'new' or 'tested', got {target.value!r}"
        )
    return target
