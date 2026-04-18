"""Audit one or more cards: spawn the auditor agent, mint tickets.

Fetches oracle text via `new_pipeline.oracle`, builds a prompt from
`prompts/auditor.md`, runs the agent with retry via `run_agent_loop`,
and hands the resulting `AuditReport` to `mint_tickets()` with audit
run metadata stamped onto every new ticket.
"""

from __future__ import annotations

import os

from new_pipeline import oracle, sandbox, utils
from new_pipeline.agent import run_agent_loop, single_file_loader
from new_pipeline.types import AuditReport, Ticket


def cmd_audit(args) -> None:
    """Entry point for `./new_pipeline/cli.py audit`."""
    cards = [c.strip() for c in args.cards.split(",") if c.strip()]
    if not cards:
        raise ValueError("--cards needs at least one non-empty name")

    total = 0
    failures: list[str] = []
    for card in cards:
        minted = _audit_one(card, args)
        if minted is None:
            failures.append(card)
        else:
            total += len(minted)

    print(
        f"\nAudit done: {total} ticket(s) minted across "
        f"{len(cards) - len(failures)}/{len(cards)} card(s)."
    )
    if failures:
        print(f"  Failed: {failures}")


def _audit_one(card: str, args) -> list[Ticket] | None:
    """Run one audit; return the tickets minted, or None on failure."""
    oracle_text = oracle.get_oracle_text(card)
    snake = utils.card_to_snake(card)
    run_id = f"{utils.today()}-{snake}-audit"

    utils.STAGING_DIR.mkdir(parents=True, exist_ok=True)
    staging_path = utils.STAGING_DIR / f"{run_id}.json"
    sandbox_path = utils.STAGING_DIR / f"{run_id}.srt.json"
    sandbox_path.write_text(
        sandbox.render(
            "auditor",
            project_root=str(utils.PROJECT_ROOT),
            home=os.path.expanduser("~"),
        )
    )

    template = (utils.PROMPTS_DIR / "auditor.md").read_text()

    def build_prompt(retry_note: str, _attempt: int) -> str:
        return template.format(
            card=card, oracle=oracle_text, staging_path=str(staging_path),
        ) + retry_note

    print(f"\n[{card}] Running audit agent...")
    report, result = run_agent_loop(
        build_prompt=build_prompt,
        cwd=utils.PROJECT_ROOT,
        load_result=single_file_loader(
            staging_path, AuditReport.load,
            missing_hint="Write the findings JSON to the staging path above.",
        ),
        model=args.model,
        effort=args.effort,
        sandbox_settings_path=sandbox_path,
    )

    if result.is_error:
        print(
            f"[{card}] FAILED: {result.error_message} "
            f"({result.duration}s, {result.tokens} tok)"
        )
        return None

    minted = report.mint_tickets(
        run_id=run_id,
        model=args.model,
        tokens=result.tokens,
        duration=result.duration,
    )
    if report.insights:
        _append_insights(report.insights, card=card)
    print(
        f"[{card}] Done: {len(minted)} ticket(s) "
        f"({result.duration}s, {result.tokens} tok)"
    )
    return minted


def _append_insights(insights: list[str], *, card: str) -> None:
    """Append agent-discovered insights to `prompts/auditor-insights.md`.

    Each insight is the markdown block the auditor produced (typically
    a `## Title` heading + paragraph). We append them verbatim so the
    next auditor reads them as part of its pre-reading.
    """
    path = utils.PROMPTS_DIR / "auditor-insights.md"
    blocks = [
        f"\n{ins.rstrip()}\n\n_Discovered auditing: {card}_\n"
        for ins in insights
    ]
    with path.open("a") as f:
        f.write("".join(blocks))
    print(
        f"[{card}] Appended {len(insights)} insight(s) to "
        f"prompts/auditor-insights.md"
    )
