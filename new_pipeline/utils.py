"""Filesystem paths shared across the package."""

from __future__ import annotations

from pathlib import Path

_HERE = Path(__file__).resolve().parent

TICKETS_DIR = _HERE / "tickets"
ARCHIVE_DIR = TICKETS_DIR / "archive"
