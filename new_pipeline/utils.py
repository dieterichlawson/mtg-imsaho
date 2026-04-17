"""Filesystem paths + small helpers shared across the package."""

from __future__ import annotations

import importlib.util
import re
from datetime import datetime
from pathlib import Path
from types import ModuleType

_HERE = Path(__file__).resolve().parent
PROJECT_ROOT = _HERE.parent

TICKETS_DIR = _HERE / "tickets"
ARCHIVE_DIR = TICKETS_DIR / "archive"
STAGING_DIR = _HERE / "staging"
PROMPTS_DIR = _HERE / "prompts"

_ORACLE_SCRIPT = PROJECT_ROOT / "scripts" / "oracle_lookup.py"


def today() -> str:
    """Today's local date as YYYY-MM-DD (used in run ids)."""
    return datetime.now().strftime("%Y-%m-%d")


def card_to_snake(name: str) -> str:
    """Normalize a card name to a snake_case slug for ticket ids."""
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


def get_oracle_text(card_name: str) -> str:
    """Return a formatted oracle-text block for `card_name` from the cache.

    Raises RuntimeError if the card isn't in `data/oracle_cache.json`
    — audit can't meaningfully run without context, and the cache is
    populated through `scripts/oracle_lookup.py add-card`.
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
    return _format_card(card, rulings or [])


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


def _format_card(card: dict, rulings: list[dict]) -> str:
    """Render a cached card entry as the text block the audit prompt gets."""
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
