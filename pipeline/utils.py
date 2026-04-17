"""Small helpers shared across the pipeline package.

Gathers the things that would otherwise each have a three-line module:
path constants, ISO timestamps, a fan-out helper, oracle lookup,
and slug validation.
"""

from __future__ import annotations

import concurrent.futures
import re
import subprocess
from collections.abc import Callable
from datetime import datetime, timezone
from pathlib import Path
from typing import TypeVar

# ── Paths ──────────────────────────────────────────────────────────

PROJECT_ROOT = Path(__file__).resolve().parent.parent
PIPELINE_DIR = PROJECT_ROOT / "pipeline"
TICKETS_DIR = PIPELINE_DIR / "tickets"
ARCHIVE_DIR = TICKETS_DIR / "archive"
STAGING_DIR = PIPELINE_DIR / "staging"
PROMPTS_DIR = PIPELINE_DIR / "prompts"
SCRIPTS_DIR = PIPELINE_DIR / "scripts"
METRICS_DIR = PIPELINE_DIR / "metrics"
LOGS_DIR = PIPELINE_DIR / "logs"
WORKTREES_DIR = PROJECT_ROOT / ".worktrees"
ORACLE_SCRIPT = PROJECT_ROOT / "scripts" / "oracle_lookup.py"

# Passed to `claude --settings` for agent subprocesses.
AGENT_SETTINGS = PIPELINE_DIR / "agent-settings.json"


# ── Timestamps ─────────────────────────────────────────────────────


def now_iso() -> str:
    """Return the current UTC ISO-8601 timestamp."""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def today() -> str:
    """Return today's local date as YYYY-MM-DD (used in run_ids)."""
    return datetime.now().strftime("%Y-%m-%d")


# ── Parallelism ────────────────────────────────────────────────────

T = TypeVar("T")
R = TypeVar("R")


def run_in_parallel(
    fn: Callable[[T], R], items: list[T], parallelism: int = 1
) -> list[R]:
    """Run fn over items, optionally in a thread pool.

    Results come back in completion order when parallelism > 1.
    """
    if parallelism <= 1 or len(items) <= 1:
        return [fn(item) for item in items]
    with concurrent.futures.ThreadPoolExecutor(max_workers=parallelism) as pool:
        futures = [pool.submit(fn, item) for item in items]
        return [f.result() for f in concurrent.futures.as_completed(futures)]


# ── Oracle lookup ──────────────────────────────────────────────────


def get_oracle_text(card_name: str) -> str | None:
    """Return oracle text + rulings for a card, or None on lookup failure."""
    for verb in ("lookup", "fetch"):
        r = subprocess.run(
            ["python3", str(ORACLE_SCRIPT), verb, card_name],
            capture_output=True,
            text=True,
            cwd=str(PROJECT_ROOT),
            check=False,
        )
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
    return None


# ── Slug validation ────────────────────────────────────────────────

_SLUG_PATTERNS = {
    "kebab": re.compile(r"[a-z0-9][a-z0-9-]*$"),
    "snake": re.compile(r"[a-z0-9_][a-z0-9_]*$"),
}


def parse_slug(value: str, *, kind: str) -> str:
    """Validate a slug against `kind` ('kebab' or 'snake'); raise if bad.

    Returns the slug unchanged on success — this reads like a parser,
    not a checker, so callers can write `slug = parse_slug(raw, ...)`.
    """
    pat = _SLUG_PATTERNS[kind]
    if not pat.fullmatch(value):
        raise ValueError(f"slug must be {kind}-case, got {value!r}")
    return value
