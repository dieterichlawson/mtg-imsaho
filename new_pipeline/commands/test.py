"""Test-writer command — run the test agent on one or more `new` tickets.

For each ticket:
    load → guard status == NEW → ensure worktree → build prompt from
    `prompts/test-writer.md` → run_agent_loop (loader parses TestReport
    + validates each confirmed test via cargo) → `t.mark_tested` or
    `t.mark_could_not_confirm` depending on the outcome.

On success the ticket moves to `tested`. If the agent rejects every
scenario, the ticket moves to the terminal `could_not_confirm` status
(archived). Agent infrastructure errors leave the ticket `new` so a
rerun can pick up where it left off.
"""

from __future__ import annotations

from new_pipeline import oracle, utils, validate, worktree
from new_pipeline.agent import run_agent_loop, single_file_loader
from new_pipeline.types import (
    Status,
    TestReport,
    TestStatus,
    Ticket,
)


def cmd_test(args) -> None:
    """Entry point for `./new_pipeline/cli.py test`."""
    ids = [i.strip() for i in args.tickets.split(",") if i.strip()]
    if not ids:
        raise ValueError("--tickets needs at least one non-empty id")
    for tid in ids:
        _test_one(tid, args)


def _test_one(tid: str, args) -> None:
    t = Ticket.load(tid)
    if t.status is not Status.NEW:
        print(
            f"[{tid}] Skip — status is {t.status.value}, not "
            f"{Status.NEW.value}"
        )
        return

    wt = worktree.ensure(tid)
    oracle_text = oracle.get_oracle_text(t.frontmatter.card)
    run_id = f"{utils.today()}-{tid}-test"
    test_file_rel = (
        f"mtg-engine/tests/pipeline_bugs_{tid.replace('-', '_')}.rs"
    )

    utils.STAGING_DIR.mkdir(parents=True, exist_ok=True)
    staging_path = utils.STAGING_DIR / f"{run_id}.json"

    template = (utils.PROMPTS_DIR / "test-writer.md").read_text()

    def build_prompt(retry_note: str, _attempt: int) -> str:
        return template.format(
            ticket_body=t.body,
            card=t.frontmatter.card,
            oracle=oracle_text,
            test_file=test_file_rel,
            staging_path=str(staging_path),
        ) + retry_note

    def _validate_tests(report: TestReport) -> str | None:
        # Every confirmed test must actually compile + fail with an
        # assertion. One bad confirmed → retry the whole run.
        for r in report.tests:
            if r.status is not TestStatus.CONFIRMED:
                continue
            why = validate.validate_test(wt, report.test_file, r.test_name)
            if why is not None:
                return (
                    f"test {r.slug!r} ({r.test_name}) failed validation: "
                    f"{why}"
                )
        return None

    print(f"\n[{tid}] Running test-writer agent...")
    report, result = run_agent_loop(
        build_prompt=build_prompt,
        cwd=wt,
        load_result=single_file_loader(
            staging_path, TestReport.load,
            validator=_validate_tests,
            missing_hint="Write the report JSON to the staging path above.",
        ),
        model=args.model,
        effort=args.effort,
    )

    if result.is_error:
        print(
            f"[{tid}] FAILED: {result.error_message} "
            f"({result.duration}s, {result.tokens} tok)"
        )
        return

    confirmed = [r for r in report.tests if r.status is TestStatus.CONFIRMED]
    if not confirmed:
        t.mark_could_not_confirm(report)
        t.save()
        print(
            f"[{tid}] {Status.COULD_NOT_CONFIRM.value}: "
            f"{len(report.tests)} scenario(s) rejected by agent."
        )
        return

    t.mark_tested(
        report,
        run_id=run_id,
        model=args.model,
        tokens=result.tokens,
        duration=result.duration,
        tested_sha=worktree.branch_head(worktree.branch_for(t.id)),
    )
    t.save()
    print(
        f"[{tid}] Done: {len(confirmed)}/{len(report.tests)} tests "
        f"confirmed ({result.duration}s, {result.tokens} tok)"
    )
