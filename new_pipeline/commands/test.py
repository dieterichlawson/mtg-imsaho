"""Test-writer command — run the test agent on one or more `new` tickets.

For each ticket:
    load → guard status == NEW → ensure worktree → build prompt from
    `prompts/test-writer.md` → run_agent_loop (loader parses TestReport
    + validates each confirmed test via cargo) → write results section
    + test-phase frontmatter → save.

On success the ticket moves to `tested`. If the agent rejects every
scenario (no real bug), the ticket keeps its `new` status — the user
decides whether to close it as false-positive in a later PR.
"""

from __future__ import annotations

from datetime import datetime, timezone

from new_pipeline import oracle, utils, validate, worktree
from new_pipeline.agent import AgentResult, run_agent_loop
from new_pipeline.types import (
    StagingError,
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

    def load_result(result: AgentResult, _attempt):
        if result.is_error:
            return None, f"agent error: {result.error_message}"
        if not staging_path.exists():
            return None, (
                f"agent did not write {staging_path.name}. "
                f"Write the report JSON to the staging path above."
            )
        try:
            report = TestReport.load(staging_path)
        except StagingError as e:
            return None, f"staging JSON failed validation: {e}"
        # Every confirmed test must actually compile + fail with an
        # assertion. One bad confirmed → retry the whole run.
        for r in report.tests:
            if r.status is not TestStatus.CONFIRMED:
                continue
            v = validate.validate_test(wt, report.test_file, r.test_name)
            if not v.ok:
                tail = v.output[-800:] if v.output else ""
                return None, (
                    f"test {r.slug!r} ({r.test_name}) failed validation: "
                    f"{v.reason}\n{tail}"
                )
        return report, None

    print(f"\n[{tid}] Running test-writer agent...")
    report, result = run_agent_loop(
        build_prompt=build_prompt,
        cwd=wt,
        load_result=load_result,
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
        # Agent rejected every scenario — no bug was found. Leave the
        # ticket in `new` for the human to triage.
        t.body = (
            t.body.rstrip() + "\n\n" + _render_results_section(report) + "\n"
        )
        t.save()
        print(
            f"[{tid}] No bugs confirmed ({len(report.tests)} rejected). "
            f"Ticket stays {Status.NEW.value}."
        )
        return

    _mark_tested(t, report, result, run_id, args, wt)
    print(
        f"[{tid}] Done: {len(confirmed)}/{len(report.tests)} tests "
        f"confirmed ({result.duration}s, {result.tokens} tok)"
    )


def _mark_tested(
    t: Ticket,
    report: TestReport,
    result: AgentResult,
    run_id: str,
    args,
    wt,
) -> None:
    """Transition the ticket to `tested` with full test-phase metadata."""
    t.status = Status.TESTED
    fm = t.frontmatter
    fm.test_run_id = run_id
    fm.test_model = args.model
    fm.test_tokens = result.tokens
    fm.test_duration = result.duration
    fm.test_file = report.test_file
    fm.tested_sha = worktree.branch_head(worktree.branch_for(t.id))
    fm.tested_at = datetime.now(timezone.utc)
    t.body = t.body.rstrip() + "\n\n" + _render_results_section(report) + "\n"
    t.save()


def _render_results_section(report: TestReport) -> str:
    """Append a `## Test Run Results` block summarizing each scenario."""
    lines = ["## Test Run Results", ""]
    for r in report.tests:
        lines.append(f"- **{r.slug}** — {r.status.value}")
        if r.test_name and r.test_name != r.slug:
            lines.append(f"  - test fn: `{r.test_name}`")
        if r.assertion_message:
            lines.append(f"  - assertion: {r.assertion_message}")
        if r.explanation:
            lines.append(f"  - explanation: {r.explanation}")
    return "\n".join(lines)
