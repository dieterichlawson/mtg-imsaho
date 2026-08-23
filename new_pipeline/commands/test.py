"""Test-writer command — run the test agent on one or more `new` tickets.

For each ticket: load → guard status == NEW → ensure worktree → build
prompt from `prompts/test-writer.md` → run_agent_loop (loader parses
TestReport + validates each confirmed test via cargo). Aggregation of
per-scenario verdicts routes the ticket to one of four destinations:

- All `confirmed` → `tested` (proceeds to fix phase).
- All `rejected` → `closed` (reason `not_a_bug`; no retry helps).
- All `needs_engine_work` → `engine_blocked` (retry grants engine sandbox).
- Any mix → `mixed` (terminal; operator hand-authors fresh tickets for
  whichever subset is worth pursuing — `retry` will not auto-mint).

Agent infrastructure errors leave the ticket `new` so a rerun can
pick up where it left off.
"""

from __future__ import annotations

import os

from new_pipeline import oracle, sandbox, utils, validate, worktree
from new_pipeline.agent import run_agent_loop, single_file_loader
from new_pipeline.types import (
    Status,
    TestReport,
    TestStatus,
    Ticket,
)


def cmd_test(args) -> None:
    """Entry point for `./new_pipeline/cli.py test`."""
    ids = utils.split_csv(args.tickets)
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

    # Retrying a `could_not_confirm` predecessor implies the previous
    # test-writer ran out of options without engine surface — grant the
    # successor write access to `mtg-engine/src/`. Fresh tickets stay
    # locked out.
    profile = (
        "test_writer_engine" if t.frontmatter.retry_of else "test_writer"
    )
    sandbox_path = utils.STAGING_DIR / f"{run_id}.srt.json"
    sandbox_path.write_text(
        sandbox.render(
            profile,
            project_root=str(utils.PROJECT_ROOT),
            home=os.path.expanduser("~"),
            test_file=test_file_rel,
        )
    )

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
        sandbox_settings_path=sandbox_path,
        log_dir=utils.LOGS_DIR,
        log_stem=run_id,
        progress_prefix=f"[{tid}] ",
        # Share the cargo target dir with the main repo so the agent's
        # first cargo invocation finds a populated cache instead of
        # cold-building all 250 cards (which would blow past the 10-min
        # Bash-tool ceiling and leave the agent spinning).
        extra_env={"CARGO_TARGET_DIR": str(utils.PROJECT_ROOT / "target")},
    )

    if result.is_error:
        print(
            f"[{tid}] FAILED: {result.error_message} "
            f"({result.duration}s, {result.tokens} tok)"
        )
        return

    # Aggregate per-scenario verdicts into the ticket's next status.
    # Status encodes what operator action is needed: mixed → review;
    # engine_blocked → retry with engine sandbox; closed → nothing.
    if not report.tests:
        # Empty report — treat as mixed so the operator looks closer.
        t.mark_mixed()
        t.append_body_section(report.to_results_section())
        t.save()
        print(f"[{tid}] {Status.MIXED.value}: report had no scenarios.")
        return

    statuses = {r.status for r in report.tests}
    total = len(report.tests)

    if statuses == {TestStatus.CONFIRMED}:
        # fall through to the mark_tested path below
        pass
    elif statuses == {TestStatus.REJECTED}:
        t.mark_not_a_bug()
        t.append_body_section(report.to_results_section())
        t.save()
        print(
            f"[{tid}] {Status.CLOSED.value} (not_a_bug): "
            f"{total} scenario(s) rejected."
        )
        return
    elif statuses == {TestStatus.NEEDS_ENGINE_WORK}:
        t.mark_engine_blocked()
        t.append_body_section(report.to_results_section())
        t.save()
        print(
            f"[{tid}] {Status.ENGINE_BLOCKED.value}: "
            f"{total} scenario(s) need engine surface. "
            f"Run `retry` to re-invoke with engine-edit access."
        )
        return
    else:
        confirmed = sum(1 for r in report.tests if r.status is TestStatus.CONFIRMED)
        t.mark_mixed()
        t.append_body_section(report.to_results_section())
        t.save()
        print(
            f"[{tid}] {Status.MIXED.value}: "
            f"{confirmed}/{total} scenario(s) confirmed. "
            f"Read the per-scenario explanations and file fresh tickets "
            f"for whichever subset is worth pursuing — retry won't auto-mint."
        )
        return

    t.mark_tested(
        run_id=run_id,
        model=args.model,
        tokens=result.tokens,
        duration=result.duration,
        test_file=report.test_file,
        tested_sha=worktree.branch_head(worktree.branch_for(t.id)),
    )
    t.append_body_section(report.to_results_section())
    t.save()
    print(
        f"[{tid}] Done: {total}/{total} tests "
        f"confirmed ({result.duration}s, {result.tokens} tok)"
    )
