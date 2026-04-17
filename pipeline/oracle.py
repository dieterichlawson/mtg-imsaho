"""Scryfall oracle text lookup — delegates to scripts/oracle_lookup.py.

`get_oracle_text` returns the full `lookup` output including name, mana
cost, type line, oracle text, keywords, and every cached ruling. The
auditor prompt treats the rulings block as first-class input — each
ruling must have a corresponding test (check 8j).
"""
from __future__ import annotations

import subprocess

from pipeline import paths


def get_oracle_text(card_name: str) -> str | None:
    """Return oracle text + rulings for a card, or None on lookup failure."""
    for verb in ("lookup", "fetch"):
        r = subprocess.run(
            ["python3", str(paths.ORACLE_SCRIPT), verb, card_name],
            capture_output=True, text=True, cwd=str(paths.PROJECT_ROOT))
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
    return None
