"""Audit one or more cards: spawn the auditor agent, mint tickets.

Fetches oracle text from `data/oracle_cache.json` via the existing
`scripts/oracle_lookup.py` (imported in-process), builds a prompt from
`prompts/auditor.md`, runs the agent with retry via `run_agent_loop`,
and hands the resulting `AuditReport` to `mint_tickets()` with audit
run metadata stamped onto every new ticket.
"""

from __future__ import annotations

import importlib.util
from types import ModuleType

from new_pipeline import utils
from new_pipeline.agent import AgentResult, run_agent_loop
from new_pipeline.types import AuditReport, StagingError, Ticket

_ORACLE_SCRIPT = utils.PROJECT_ROOT / "scripts" / "oracle_lookup.py"


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
    oracle = get_oracle_text(card)
    snake = utils.card_to_snake(card)
    run_id = f"{utils.today()}-{snake}-audit"

    utils.STAGING_DIR.mkdir(parents=True, exist_ok=True)
    staging_path = utils.STAGING_DIR / f"{run_id}.json"

    template = (utils.PROMPTS_DIR / "auditor.md").read_text()

    def build_prompt(retry_note: str, _attempt: int) -> str:
        return template.format(
            card=card, oracle=oracle, staging_path=str(staging_path),
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


# ── Oracle lookup ─────────────────────────────────────────────────


def get_oracle_text(card_name: str) -> str:
    """Return a formatted oracle-text block from the cache for `card_name`.

    Raises RuntimeError if the card isn't in `data/oracle_cache.json` —
    audit can't meaningfully run without context, and the cache is
    populated via `scripts/oracle_lookup.py add-card`.
    """
    ol = _oracle_module()
    cache = ol.load_cache()
    _, card = ol.find_card(cache, card_name)
    if card is None:
        raise RuntimeError(
            f"no oracle text for {card_name!r} — "
            f"try `scripts/oracle_lookup.py add-card '{card_name}'` first"
        )
    _, rulings = ol.find_rulings(cache, card_name)
    return _format_card_for_prompt(card, rulings or [])


_oracle_lookup: ModuleType | None = None


def _oracle_module() -> ModuleType:
    """Load `scripts/oracle_lookup.py` as a module the first time, cache it."""
    global _oracle_lookup
    if _oracle_lookup is None:
        spec = importlib.util.spec_from_file_location(
            "new_pipeline._oracle_lookup", _ORACLE_SCRIPT,
        )
        assert spec is not None and spec.loader is not None
        _oracle_lookup = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_oracle_lookup)
    return _oracle_lookup


def _format_card_for_prompt(card: dict, rulings: list[dict]) -> str:
    """Render a cached card entry as the text block embedded in the prompt."""
    lines = [f"Name: {card['name']}"]
    if card.get("mana_cost"):
        lines.append(f"Mana Cost: {card['mana_cost']}")
    lines.append(f"Type Line: {card.get('type_line', 'N/A')}")
    if card.get("power") or card.get("toughness"):
        lines.append(
            f"P/T: {card.get('power', '?')}/{card.get('toughness', '?')}"
        )
    lines.append(f"Oracle Text: {card.get('oracle_text', 'N/A')}")
    if card.get("back_face"):
        bf = card["back_face"]
        lines += ["", "--- Back Face ---", f"Name: {bf['name']}"]
        lines.append(f"Type Line: {bf.get('type_line', 'N/A')}")
        if bf.get("power") or bf.get("toughness"):
            lines.append(
                f"P/T: {bf.get('power', '?')}/{bf.get('toughness', '?')}"
            )
        lines.append(f"Oracle Text: {bf.get('oracle_text', 'N/A')}")
    if rulings:
        lines += ["", f"--- Rulings ({len(rulings)}) ---"]
        for r in rulings:
            lines.append(f"[{r.get('date', '?')}] {r['text']}")
    return "\n".join(lines)
