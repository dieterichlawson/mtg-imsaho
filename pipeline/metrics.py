"""Append-only metrics logs.

`runs.jsonl` gets one entry per agent invocation; `findings.jsonl` gets
one per ticket state transition. Both files are consumed by
`scripts/metrics.py` and `scripts/report.py`.
"""

from __future__ import annotations

import json

from pipeline import utils
from pipeline.utils import now_iso


def _append(name: str, entry: dict) -> None:
    utils.METRICS_DIR.mkdir(parents=True, exist_ok=True)
    with open(utils.METRICS_DIR / name, "a") as f:
        f.write(json.dumps(entry) + "\n")


def log_run(
    role: str,
    *,
    run_id: str,
    model: str,
    card: str,
    result: dict,
    validation_passed: bool = True,
    finding_id: str | None = None,
    findings_created: int = 0,
    test_result: str | None = None,
    fix_result: str | None = None,
    rejection_reason: str | None = None,
    notes: str = "",
) -> None:
    """Append one `runs.jsonl` entry recording an agent invocation."""
    _append(
        "runs.jsonl",
        {
            "run_id": run_id,
            "timestamp": now_iso(),
            "role": role,
            "model": model,
            "card": card,
            "finding_id": finding_id,
            "findings_created": findings_created,
            "test_result": test_result,
            "fix_result": fix_result,
            "validation_passed": validation_passed,
            "rejection_reason": rejection_reason,
            "total_tokens": result.get("tokens", 0),
            "tool_uses": result.get("tool_uses", 0),
            "duration_seconds": result.get("duration", 0),
            "notes": notes,
        },
    )


def log_finding(
    finding_id: str, event: str, *, card: str = "", run_id: str = "", **extra
) -> None:
    """Append one `findings.jsonl` entry recording a ticket state change."""
    _append(
        "findings.jsonl",
        {
            "finding_id": finding_id,
            "timestamp": now_iso(),
            "event": event,
            "card": card,
            "source": "code-audit",
            "engine_file": "",
            "description": finding_id,
            "run_id": run_id,
            **extra,
        },
    )
