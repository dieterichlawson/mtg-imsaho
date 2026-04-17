"""Append-only metrics logs — runs.jsonl (per agent invocation) and
findings.jsonl (per ticket state transition).
"""
from __future__ import annotations

import json
from datetime import datetime, timezone

from pipeline import paths


def now_iso() -> str:
    """UTC ISO-8601 timestamp. Used in metrics entries and ticket frontmatter
    (`tested_at`, `shipped_at`, `closed_at`, etc.).
    """
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def today() -> str:
    """YYYY-MM-DD — used in run_ids and log-file names."""
    return datetime.now().strftime("%Y-%m-%d")


def _append(name: str, entry: dict) -> None:
    paths.METRICS_DIR.mkdir(parents=True, exist_ok=True)
    with open(paths.METRICS_DIR / name, "a") as f:
        f.write(json.dumps(entry) + "\n")


def log_run(role: str, *, run_id: str, model: str, card: str,
            result: dict, validation_passed: bool = True,
            finding_id: str | None = None, findings_created: int = 0,
            test_result: str | None = None, fix_result: str | None = None,
            rejection_reason: str | None = None, notes: str = "") -> None:
    """Append one `runs.jsonl` entry recording an agent invocation."""
    _append("runs.jsonl", {
        "run_id": run_id, "timestamp": now_iso(), "role": role,
        "model": model, "card": card, "finding_id": finding_id,
        "findings_created": findings_created,
        "test_result": test_result, "fix_result": fix_result,
        "validation_passed": validation_passed,
        "rejection_reason": rejection_reason,
        "total_tokens": result.get("tokens", 0),
        "tool_uses": result.get("tool_uses", 0),
        "duration_seconds": result.get("duration", 0), "notes": notes})


def log_finding(finding_id: str, event: str, *,
                card: str = "", run_id: str = "", **extra) -> None:
    """Append one `findings.jsonl` entry recording a ticket state change."""
    _append("findings.jsonl", {
        "finding_id": finding_id, "timestamp": now_iso(), "event": event,
        "card": card, "source": "code-audit", "engine_file": "",
        "description": finding_id, "run_id": run_id, **extra})
