"""Filesystem paths + small helpers shared across the package."""

from __future__ import annotations

import re
import subprocess
from datetime import datetime
from pathlib import Path

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
    """Fetch the card's oracle text via `scripts/oracle_lookup.py`.

    Raises RuntimeError if the script can't produce a text block for
    this card — audit can't meaningfully run without oracle context.
    """
    for verb in ("lookup", "fetch"):
        r = subprocess.run(
            ["python3", str(_ORACLE_SCRIPT), verb, card_name],
            capture_output=True,
            text=True,
            cwd=str(PROJECT_ROOT),
            check=False,
        )
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
    raise RuntimeError(
        f"no oracle text for {card_name!r} — "
        f"try `scripts/oracle_lookup.py add-card '{card_name}'` first"
    )
