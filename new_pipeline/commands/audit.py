"""Audit one or more cards: spawn the auditor agent, mint tickets.

Fetches oracle text via `new_pipeline.oracle`, builds a prompt from
`prompts/auditor.md`, runs the agent with retry via `run_agent_loop`,
and hands the resulting `AuditReport` to `mint_tickets()` with audit
run metadata stamped onto every new ticket.
"""

from __future__ import annotations

from new_pipeline import oracle, utils
from new_pipeline.agent import AgentResult, run_agent_loop
from new_pipeline.types import AuditReport, StagingError, Ticket


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

    template = (utils.PROMPTS_DIR / "auditor.md").read_text()

    def build_prompt(retry_note: str, _attempt: int) -> str:
        return template.format(
            card=card, oracle=oracle_text, staging_path=str(staging_path),
        ) + retry_note

    def load_result(result: AgentResult, _attempt):
        if result.is_error:
            return None, f"agent error: {result.error_message}"
        if not staging_path.exists():
            return None, (
                f"agent did not write {staging_path.name}. "
                f"Write the findings JSON to the staging path above."
            )
        try:
            return AuditReport.load(staging_path), None
        except StagingError as e:
            return None, f"staging JSON failed validation: {e}"

    print(f"\n[{card}] Running audit agent...")
    report, result = run_agent_loop(
        build_prompt=build_prompt,
        cwd=utils.PROJECT_ROOT,
        load_result=load_result,
        model=args.model,
        effort=args.effort,
    )

    if result.is_error:
        print(
            f"[{card}] FAILED: {result.error_message} "
            f"({result.duration}s, {result.tokens} tok)"
        )
        return None

    extras = {
        "audit_run_id": run_id,
        "audit_model": args.model,
        "audit_tokens": str(result.tokens),
        "audit_duration": str(result.duration),
    }
    minted = report.mint_tickets(extra=extras)
    print(
        f"[{card}] Done: {len(minted)} ticket(s) "
        f"({result.duration}s, {result.tokens} tok)"
    )
    return minted
