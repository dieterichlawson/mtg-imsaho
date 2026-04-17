"""Test-writer stage — spawn the agent, validate each emitted test, route.

the ticket to its post-test status.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from pipeline import ticket, utils, validate, worktree
from pipeline.agent import (
    MAX_ATTEMPTS,
    build_prompt,
    run_agent_loop,
    worktree_staging_file,
)
from pipeline.metrics import log_finding, log_run
from pipeline.staging import (
    TEST_BLOCKED,
    TEST_CONFIRMED,
    TEST_REJECTED,
    TestReport,
    TestResult,
)
from pipeline.state import Status, TestOutcome, next_status
from pipeline.ticket import Ticket
from pipeline.utils import now_iso, run_in_parallel, today

# Retry-feedback fragments. Surfaced to the agent as `## Retry note`.
_EMPTY_TESTS = (
    "Your staging JSON had no entries under `tests`. Emit one per slug "
    "in the ticket's `## Tests` section."
)
_FIELD_PREAMBLE = "Your staging JSON is missing required fields:\n"
_FIELD_CODA = (
    "\nConfirmed MUST have `test_name`. Blocked MUST have `blocked_by`."
)
_ENGINE_NOTE = (
    "\n### Engine edits allowed this run\n"
    "A prior attempt reported the test cannot be expressed without an "
    "engine change. For this run only, you may modify `mtg-engine/src/**` "
    "to add the minimal surface area needed. See `## Engine Change Needed` "
    "in the ticket body.\n"
)


@dataclass
class TestPhaseResult:
    """Aggregated verdict + per-test results from one test-writer run."""

    aggregate: TestOutcome = TestOutcome.NO_REAL_BUG
    per_test: list[TestResult] = field(default_factory=list)
    implementations: dict[str, str] = field(default_factory=dict)
    test_file: str = ""


def cmd_test(args):
    """Entry point for `./pipeline/cli.py test`."""
    tickets = _select(args)
    if not tickets:
        print("No tickets to test.")
        return

    print(f"\n{'=' * 60}\nTEST WRITER — {len(tickets)} ticket(s)\n{'=' * 60}")
    for t in tickets:
        print(f"  {t.id}: {t.card}")

    def run_one(t):
        return test_one(t, args)

    results = run_in_parallel(run_one, tickets, args.parallelism)
    _print_summary(results)


def _select(args) -> list[Ticket]:
    if args.tickets:
        tickets = []
        for tid in [t.strip() for t in args.tickets.split(",")]:
            t = ticket.get_ticket_if_exists(tid)
            if t is not None:
                tickets.append(t)
    else:
        tickets = ticket.list_all(status=Status.NEW)
    return [t for t in tickets if t.status is Status.NEW]


# ── One ticket, top to bottom ───────────────────────────────────────


def test_one(t: Ticket, args) -> dict:
    """Run the test-writer on `t` and route it to its post-test status."""
    wt = worktree.ensure(t.id)
    staging_file = worktree_staging_file(wt, t.id, "test")
    outcome, result = _run_test_writer(t, wt, staging_file, args)
    _apply_test_outcome(t, outcome, result, args)
    return _summarize(t, outcome, result)


def _run_test_writer(
    t: Ticket, wt: Path, staging_file: Path, args
) -> tuple[TestPhaseResult, dict]:
    oracle_text = utils.get_oracle_text(t.card) or "Oracle text not available"
    engine_note = _ENGINE_NOTE if t.allow_engine_edits else ""
    tid_snake = t.id.replace("-", "_")
    default_file = f"mtg-engine/tests/pipeline_bugs_{tid_snake}.rs"
    expected_slugs = {e.slug for e in ticket.parse_tests_section(t.body)}
    outcome = TestPhaseResult(test_file=default_file)

    builder = build_prompt(
        "test-writer",
        ticket_body=t.body,
        oracle=oracle_text,
        engine_note=engine_note,
        tid_snake=tid_snake,
        tid=t.id,
    )

    def validator(parsed, _result, attempt):
        return _validate_test_run(
            parsed, attempt, wt, default_file, expected_slugs, outcome
        )

    print(f"  [{t.id}] Spawning agent in worktree...")
    _, result = run_agent_loop(
        build_prompt=builder,
        cwd=wt,
        staging_file=staging_file,
        loader=TestReport.load,
        validator=validator,
        model=args.model,
        effort=args.effort,
        log_prefix=f"{today()}-{t.id}-test",
        progress_prefix=f"  [{t.id}] ",
    )
    return outcome, result


# ── Validation ──────────────────────────────────────────────────────


def _validate_test_run(
    parsed: TestReport,
    attempt: int,
    wt: Path,
    default_file: str,
    expected_slugs: set[str],
    out: TestPhaseResult,
) -> str | None:
    """Mutate `out` with the agent's per-test results. Return None to accept.

    or a retry-note string to ask the agent again.
    """
    if not parsed.tests:
        return _EMPTY_TESTS

    out.test_file = parsed.test_file or default_file
    out.per_test = list(parsed.tests)

    missing_fields = _check_field_completeness(out.per_test)
    if missing_fields:
        return _FIELD_PREAMBLE + "\n".join(missing_fields) + _FIELD_CODA

    out.implementations = _validate_each_test(out.per_test, wt, out.test_file)
    out.aggregate = _aggregate(out.per_test)

    # Fast accept for anything other than a clean tested run.
    if out.aggregate is not TestOutcome.CONFIRMED:
        return None

    missing = expected_slugs - {t.slug for t in out.per_test}
    if missing:
        out.aggregate = TestOutcome.NO_REAL_BUG
        if attempt < MAX_ATTEMPTS:
            return (
                f"Ticket's `## Tests` section lists {len(expected_slugs)} "
                f"slug(s) but staging has no confirmed entry for every "
                f"one. Missing: {sorted(missing)}."
            )
        return None

    dirty = worktree.is_clean(wt)
    if dirty:
        out.aggregate = TestOutcome.NO_REAL_BUG
        if attempt < MAX_ATTEMPTS:
            return (
                f"Every per-test validation succeeded but worktree "
                f"is not clean:\n\n{dirty}\n\nRun `git add -A && "
                f"git commit` before writing staging output."
            )
        return None

    return None


def _check_field_completeness(per_test: list[TestResult]) -> list[str]:
    errors = []
    for t in per_test:
        if t.status == TEST_CONFIRMED and not t.test_name:
            errors.append(f"- `{t.slug}`: confirmed but test_name empty")
        if t.status == TEST_BLOCKED and not (t.blocked_by or "").strip():
            errors.append(f"- `{t.slug}`: blocked but blocked_by empty")
    return errors


def _validate_each_test(
    per_test: list[TestResult], wt: Path, test_file: str
) -> dict[str, str]:
    """Run cargo against each confirmed test and record the implementation.

    pointer. Rejects (with note) a confirmed test that passes against current
    code — it's a false positive for that specific scenario, not a real bug.
    Returns slug → implementation string for the ticket body.
    """
    impls: dict[str, str] = {}
    for t in per_test:
        if t.status == TEST_CONFIRMED:
            result = validate.validate_test(wt, test_file, t.test_name)
            if result.is_confirmed_bug:
                impls[t.slug] = f"{test_file}::{t.test_name}"
            else:
                t.status = TEST_REJECTED
                impls[t.slug] = f"rejected: {result.reason}"
        elif t.status == TEST_REJECTED:
            impls[t.slug] = f"rejected: {t.explanation[:80]}"
        elif t.status == TEST_BLOCKED:
            impls[t.slug] = "(not yet written)"
    return impls


def _aggregate(per_test: list[TestResult]) -> TestOutcome:
    statuses = {t.status for t in per_test}
    if TEST_BLOCKED in statuses:
        return TestOutcome.NEEDS_ENGINE
    if TEST_CONFIRMED in statuses:
        return TestOutcome.CONFIRMED
    return TestOutcome.NO_REAL_BUG


# ── Apply the outcome: state, frontmatter, body, metrics ────────────


def _apply_test_outcome(
    t: Ticket, out: TestPhaseResult, result: dict, args
) -> None:
    run_id = f"{today()}-{t.id}-test"
    if out.implementations:
        ticket.set_test_implementations(t, out.implementations)
    if out.aggregate is TestOutcome.NO_REAL_BUG:
        worktree.remove(t.id)

    t.append_section(_render_results_section(out.per_test))
    _apply_status(t, out, result, args.model, run_id)
    _log(t, out, result, args.model, run_id)


def _apply_status(
    t: Ticket, out: TestPhaseResult, result: dict, model: str, run_id: str
) -> None:
    target = next_status(Status.NEW, out.aggregate)
    confirmed = sum(1 for e in out.per_test if e.status == TEST_CONFIRMED)
    fields = {
        "test_run_id": run_id,
        "test_model": model,
        "test_tokens": str(result["tokens"]),
        "test_duration": str(result["duration"]),
        "test_file": out.test_file,
        "tests_confirmed": str(confirmed),
        "tests_total": str(len(out.per_test)),
    }

    if out.aggregate is TestOutcome.CONFIRMED:
        fields.update(
            {
                "tested_at": now_iso(),
                "worktree": str(worktree.dir_for(t.id)),
                "tested_sha": worktree.branch_head(worktree.branch_for(t.id)),
            }
        )
    elif out.aggregate is TestOutcome.NEEDS_ENGINE:
        fields.update(
            {"allow_engine_edits": "true", "engine_block_at": now_iso()}
        )
    else:
        fields["false_positive_at"] = now_iso()

    t.status = target
    t.frontmatter.set(**fields)
    t.save()

    if out.aggregate is TestOutcome.NEEDS_ENGINE:
        blockers = [
            f"- **{e.slug}**: {e.blocked_by}"
            for e in out.per_test
            if e.status == TEST_BLOCKED and e.blocked_by
        ]
        if blockers:
            t.append_section(
                "## Engine Change Needed\n\n"
                "The test-writer could not express the following test(s) "
                "without engine changes. Retry will allow "
                "`mtg-engine/src/**` edits.\n\n" + "\n".join(blockers)
            )


def _render_results_section(per_test: list[TestResult]) -> str:
    lines = ["## Test Run Results", ""]
    for t in per_test:
        lines.append(f"- **{t.slug}** — {t.status}")
        if t.test_name:
            lines.append(f"  - test fn: `{t.test_name}`")
        if t.assertion_message:
            lines.append(f"  - assertion: {t.assertion_message}")
        if t.blocked_by:
            lines.append(f"  - blocked by: {t.blocked_by}")
    return "\n".join(lines)


def _log(
    t: Ticket, out: TestPhaseResult, result: dict, model: str, run_id: str
) -> None:
    validated = out.aggregate is TestOutcome.CONFIRMED
    confirmed = sum(1 for x in out.per_test if x.status == TEST_CONFIRMED)
    total = len(out.per_test)
    log_run(
        "test-writer",
        run_id=run_id,
        model=model,
        card=t.card,
        result=result,
        finding_id=t.id,
        test_result=out.aggregate.value,
        validation_passed=validated,
        rejection_reason=None
        if validated
        else "one-or-more tests failed validation",
        notes=f"{confirmed}/{total} tests confirmed",
    )
    log_finding(
        t.id,
        f"test_{out.aggregate.value}",
        card=t.card,
        run_id=run_id,
        test_file=out.test_file,
    )


def _summarize(t: Ticket, out: TestPhaseResult, result: dict) -> dict:
    confirmed = sum(1 for x in out.per_test if x.status == TEST_CONFIRMED)
    total = len(out.per_test)
    print(
        f"  [{t.id}] Done: {out.aggregate.value} — {confirmed}/{total} "
        f"tests ({result['duration']}s, {result['tokens']} tok)"
    )
    return {
        "ticket": t.id,
        "result": out.aggregate.value,
        "confirmed": confirmed,
        "total": total,
    }


def _print_summary(results: list[dict]) -> None:
    print(f"\n{'=' * 60}\nTEST SUMMARY\n{'=' * 60}")
    for r in sorted(results, key=lambda x: x["ticket"]):
        detail = f"{r['confirmed']}/{r['total']} tests" if r["total"] else ""
        print(f"  {r['ticket']:<40} {r['result']:<14} {detail}")
    counts = {"confirmed": 0, "needs_engine": 0, "no_real_bug": 0}
    for r in results:
        counts[r["result"]] = counts.get(r["result"], 0) + 1
    print(
        f"\n  Confirmed: {counts['confirmed']}  "
        f"Needs engine: {counts['needs_engine']}  "
        f"No real bug: {counts['no_real_bug']}"
    )
