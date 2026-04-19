"""Filesystem paths + small helpers shared across the package."""

from __future__ import annotations

import re
from datetime import datetime
from pathlib import Path

_HERE = Path(__file__).resolve().parent
PROJECT_ROOT = _HERE.parent

TICKETS_DIR = _HERE / "tickets"
ARCHIVE_DIR = TICKETS_DIR / "archive"
STAGING_DIR = _HERE / "staging"
PROMPTS_DIR = _HERE / "prompts"
LOGS_DIR = _HERE / "logs"
WORKTREES_DIR = PROJECT_ROOT / ".worktrees"


def today() -> str:
    """Today's local date as YYYY-MM-DD (used in run ids)."""
    return datetime.now().strftime("%Y-%m-%d")


def card_to_snake(name: str) -> str:
    """Normalize a card name to a snake_case slug for ticket ids."""
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


def split_csv(raw: str) -> list[str]:
    """Split a CLI list argument. Uses `;` if present (lets card names
    with commas like `Mikaeus, the Lunarch` pass through), otherwise
    falls back to `,`. Empty entries are dropped and whitespace stripped.
    """
    sep = ";" if ";" in raw else ","
    return [x.strip() for x in raw.split(sep) if x.strip()]
