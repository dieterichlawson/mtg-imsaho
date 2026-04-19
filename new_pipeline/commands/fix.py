"""Fixer command — run the fix agent on one or more `tested` tickets.

For each ticket:
    load → guard status == TESTED → use existing worktree (from the
    test phase) → build prompt from `prompts/fixer.md` →
    `run_agent_loop` with a loader that parses `FixReport` and, on
    `status == fixed`, runs `validate_fix` → on success transition
    via `t.mark_fixed(...)` + append the result section; on the
    agent's self-reported `failed`, transition via
    `t.mark_fix_failed(...)` + append the post-mortem.

Agent infrastructure errors (timeout, bad JSON after retries) leave
the ticket `tested` so a rerun picks up where it left off.
"""

from __future__ import annotations

import os

from new_pipeline import sandbox, utils, validate, worktree
from new_pipeline.agent import run_agent_loop, single_file_loader
from new_pipeline.types import (
    FixReport,
    FixStatus,
    Status,
    Ticket,
)


def cmd_fix(args) -> None:
    """Entry point for `./new_pipeline/cli.py fix`."""
    ids = [i.strip() for i in args.tickets.split(",") if i.strip()]
    if not ids:
        raise ValueError("--tickets needs at least one non-empty id")
    for tid in ids:
        _fix_one(tid, args)


def _fix_one(tid: str, args) -> None:
    t = Ticket.load(tid)
    if t.status is not Status.TESTED:
        print(
            f"[{tid}] Skip — status is {t.status.value}, not "
            f"{Status.TESTED.value}"
        )
        return

    wt = worktree.dir_for(t.id)
    if not wt.exists():
        raise RuntimeError(
            f"[{t.id}] no worktree at {wt} — the test phase must run first"
        )

    tested_sha = t.frontmatter.tested_sha
    test_file_rel = t.frontmatter.test_file
    run_id = f"{utils.today()}-{tid}-fix"

    utils.STAGING_DIR.mkdir(parents=True, exist_ok=True)
    staging_path = utils.STAGING_DIR / f"{run_id}.json"
    sandbox_path = utils.STAGING_DIR / f"{run_id}.srt.json"
    sandbox_path.write_text(
        sandbox.render(
            "fixer",
            project_root=str(utils.PROJECT_ROOT),
            home=os.path.expanduser("~"),
        )
    )

    template = (utils.PROMPTS_DIR / "fixer.md").read_text()

    def build_prompt(retry_note: str, _attempt: int) -> str:
        return template.format(
            ticket_body=t.body,
            test_file=test_file_rel,
            tested_sha=tested_sha,
            staging_path=str(staging_path),
        ) + retry_note

    def _validate(report: FixReport) -> str | None:
        # Agent-reported `failed` needs no cargo check — accept the
        # post-mortem. Only `fixed` reports get verified.
        if report.status is FixStatus.FAILED:
            return None
        return validate.validate_fix(wt, tested_sha)

    print(f"\n[{tid}] Running fixer agent...")
    report, result = run_agent_loop(
        build_prompt=build_prompt,
        cwd=wt,
        load_result=single_file_loader(
            staging_path, FixReport.load,
            validator=_validate,
            missing_hint="Write the fix report JSON to the staging path above.",
        ),
        model=args.model,
        effort=args.effort,
        sandbox_settings_path=sandbox_path,
        log_dir=utils.LOGS_DIR,
        log_stem=run_id,
        progress_prefix=f"[{tid}] ",
        # Share the cargo target dir with the main repo so the agent's
        # first cargo invocation finds a populated cache instead of
        # cold-building all 250 cards.
        extra_env={"CARGO_TARGET_DIR": str(utils.PROJECT_ROOT / "target")},
    )

    if result.is_error:
        print(
            f"[{tid}] FAILED: {result.error_message} "
            f"({result.duration}s, {result.tokens} tok)"
        )
        return

    if report.status is FixStatus.FIXED:
        t.mark_fixed(
            run_id=run_id,
            model=args.model,
            tokens=result.tokens,
            duration=result.duration,
            fixed_sha=worktree.branch_head(worktree.branch_for(t.id)),
        )
        t.append_body_section(report.to_result_section())
        t.save()
        print(
            f"[{tid}] {Status.FIXED.value} "
            f"({result.duration}s, {result.tokens} tok)"
        )
        return

    # Agent-reported failed — record the post-mortem.
    t.mark_fix_failed(
        run_id=run_id,
        model=args.model,
        tokens=result.tokens,
        duration=result.duration,
    )
    t.append_body_section(report.to_result_section())
    t.save()
    print(
        f"[{tid}] {Status.FIX_FAILED.value} — agent gave up "
        f"({result.duration}s, {result.tokens} tok)"
    )
