"""Oracle-text lookup — shared prompt context for agent-driven commands.

`scripts/oracle_lookup.py` lives outside `new_pipeline` but is importable
Python. This module wraps it: loads the script once via importlib,
queries the cached card, and renders the result as the text block
embedded in agent prompts.

Both the audit command and the test-writer command want the same block,
so it lives here rather than inside either one.
"""

from __future__ import annotations

import importlib.util
from types import ModuleType

from new_pipeline import utils

_ORACLE_SCRIPT = utils.PROJECT_ROOT / "scripts" / "oracle_lookup.py"


def get_oracle_text(card_name: str) -> str:
    """Return a formatted oracle-text block from the cache for `card_name`.

    Raises RuntimeError if the card isn't in `data/oracle_cache.json` —
    agents can't meaningfully work without context, and the cache is
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


# ── Internals ─────────────────────────────────────────────────────

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
    """Render a cached card entry as the text block embedded in a prompt."""
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
