"""Fixer stage — spawn the fixer agent, validate the fix, route the ticket."""

from __future__ import annotations

import sys
from pathlib import Path

from pipeline import validate, worktree
from pipeline.agent import (
    MAX_ATTEMPTS,
    build_prompt,
    run_agent_loop,
    single_file_loader,
    worktree_staging_file,
)
from pipeline.metrics import log_finding, log_run
from pipeline.models import FixReport, Ticket, parse_tests_section
from pipeline.state import FixOutcome, Status, next_status
from pipeline.utils import now_iso, today

# Retry-feedback fragment surfaced to the agent.
_NEEDS_POSTMORTEM = (
    "Previous attempt reported status=failed but omitted `description`. "
    "A post-mortem is the only artifact of a failed run — explain what "
    "you tried and what engine-level change would be required."
)


def cmd_fix(args):
    """Entry point for `./pipeline/cli.py fix`."""
    t = _pick_ticket(args)
    if t is None:
        print(f"No {Status.TESTED.value} tickets to fix.")
        return

    print(f"\n{'=' * 60}\nFIXER — {t.id} ({t.card})\n{'=' * 60}")
    fix_one(t, args)


def _pick_ticket(args) -> Ticket | None:
    if args.ticket:
        t = Ticket.load(args.ticket)
        if t.status is not Status.TESTED:
            print(
                f"Ticket {t.id}: status={t.status.value}, not "
                f"{Status.TESTED.value}. Use `retry --to tested` to rewind."
            )
            sys.exit(1)
        return t
    candidates = Ticket.list_all(status=Status.TESTED)
    return candidates[0] if candidates else None


def fix_one(t: Ticket, args) -> None:
    """Run the fixer on `t` and route it to fixed or fix_failed."""
    wt = worktree.dir_for(t.id)
    if not wt.exists():
        print(f"  No worktree found for {t.id}. Run `test` first.")
        sys.exit(1)

    outcome, parsed, result = _run_fixer(t, wt, args)
    _apply_fix_outcome(t, outcome, parsed, result, args)

    if outcome is FixOutcome.FAILED:
        print(f"  Worktree preserved for inspection: {wt}")
        print(
            f"  Run `./pipeline/cli.py retry {t.id}` to rewind, or "
            f"`close {t.id}` to give up."
        )
    print(
        f"\n  Result: {outcome.value} "
        f"({result['duration']}s, {result['tokens']} tok)"
    )


def _run_fixer(t: Ticket, wt: Path, args) -> tuple[FixOutcome, dict, dict]:
    staging_file = worktree_staging_file(wt, t.id, "fix")
    failing_block = _failing_tests_block(t)
    builder = build_prompt(
        "fixer",
        ticket_body=t.body,
        num_tests=failing_block.count("\n- `") + 1,
        test_file=t.test_file,
        failing_tests_block=failing_block,
        tid=t.id,
    )

    def validator(parsed, _result, attempt):
        return _validate_fix_run(parsed, attempt, wt)

    print(f"  Spawning agent in worktree {wt.name}...")
    parsed, result = run_agent_loop(
        build_prompt=builder,
        cwd=wt,
        load_result=single_file_loader(
            staging_file, FixReport.load, validator
        ),
        model=args.model,
        effort=args.effort,
        log_prefix=f"{today()}-{t.id}-fix",
        progress_prefix=f"  [{t.id}] ",
    )

    if parsed is None:
        parsed = FixReport(status="failed", files_changed=[], description="")
    outcome = _interpret_outcome(parsed, wt)
    return outcome, parsed, result


def _failing_tests_block(t: Ticket) -> str:
    fns = [
        impl.split("::", 1)[1]
        for entry in parse_tests_section(t.body)
        if "::" in (impl := entry.implementation.strip())
    ]
    if not fns:
        return "- (see ticket `## Tests` section)"
    return "\n".join(f"- `{n}`" for n in fns)


def _validate_fix_run(parsed: FixReport, attempt: int, wt: Path) -> str | None:
    """Accept the agent's "fixed" claim only if validate_fix concurs.

    Require a post-mortem when the agent reports "failed".
    """
    if parsed.status != "fixed":
        if not parsed.description and attempt < MAX_ATTEMPTS:
            return _NEEDS_POSTMORTEM
        return None

    v = validate.validate_fix(wt)
    if v.ok:
        return None
    tail = (v.output or "")[-1500:]
    print(f"  Validation FAILED (attempt {attempt}): {v.reason}\n{tail}")
    return (
        f"validate_fix rejected your previous attempt: {v.reason}\n\n"
        f"Output (last 1500 chars):\n{tail}\n\nFix and retry. Remember "
        f"to commit changes before running validate_fix."
    )


def _interpret_outcome(parsed: FixReport, wt: Path) -> FixOutcome:
    """Final verdict after the loop ends. The loop's validator has already.

    consented; we re-check here for belt-and-braces.
    """
    if parsed.status != "fixed":
        return FixOutcome.FAILED
    v = validate.validate_fix(wt)
    return FixOutcome.SUCCEEDED if v.ok else FixOutcome.FAILED


def _apply_fix_outcome(
    t: Ticket, outcome: FixOutcome, parsed: FixReport, result: dict, args
) -> None:
    run_id = f"{today()}-{t.id}-fix"
    t.status = next_status(Status.TESTED, outcome)
    status_key = t.status.value
    t.append_section(_render_fix_section(status_key, parsed))
    t.frontmatter.set(
        **{
            f"{status_key}_at": now_iso(),
            "fix_run_id": run_id,
            "fix_model": args.model,
            "fix_tokens": str(result["tokens"]),
            "fix_duration": str(result["duration"]),
        }
    )
    if t.status is Status.FIXED:
        t.frontmatter.fixed_sha = worktree.branch_head(
            worktree.branch_for(t.id)
        )
    t.save()

    validated = outcome is FixOutcome.SUCCEEDED
    log_run(
        "fixer",
        run_id=run_id,
        model=args.model,
        card=t.card,
        result=result,
        finding_id=t.id,
        fix_result="fixed" if validated else "failed",
        validation_passed=validated,
        rejection_reason=None if validated else "validation failed",
    )
    log_finding(
        t.id,
        "fix_succeeded" if validated else "fix_failed",
        card=t.card,
        run_id=run_id,
    )


def _render_fix_section(status_key: str, parsed: FixReport) -> str:
    lines = [f"## Fix Result\n\nstatus: {status_key}"]
    if parsed.files_changed:
        lines.append("files_changed:")
        lines += [f"- {p}" for p in parsed.files_changed]
    if parsed.description:
        lines += ["", parsed.description]
    return "\n".join(lines)
