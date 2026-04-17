"""Staging JSON loaders — validate and normalize the agent→Python wire."""
from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any


class StagingError(ValueError):
    """Raised when a staging JSON file's shape is invalid."""


def _load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as e:
        raise StagingError(f"{path.name}: invalid JSON — {e}") from None


def _require(d: dict, field: str, kind: type) -> Any:
    if field not in d:
        raise StagingError(f"missing required field: {field!r}")
    v = d[field]
    if not isinstance(v, kind):
        raise StagingError(f"field {field!r}: expected {kind.__name__}, "
                           f"got {type(v).__name__}")
    return v


def _require_objects(d: dict, field: str) -> list[dict]:
    arr = _require(d, field, list)
    for i, x in enumerate(arr):
        if not isinstance(x, dict):
            raise StagingError(
                f"{field}[{i}]: expected object, got {type(x).__name__}")
    return arr


# Per-test statuses — re-exported by cli.py for tests.
TEST_CONFIRMED = "confirmed"
TEST_REJECTED  = "rejected"
TEST_BLOCKED   = "blocked"
VALID_TEST_STATUSES = {TEST_CONFIRMED, TEST_REJECTED, TEST_BLOCKED}


def load_audit(path: Path) -> dict:
    d = _load_json(path)
    findings = []
    for f in _require_objects(d, "findings"):
        tests = [{"slug": _require(t, "slug", str),
                  "scenario": _require(t, "scenario", str)}
                 for t in (f.get("tests") or [])]
        findings.append({
            "oracle_quote":   _require(f, "oracle_quote", str),
            "code_quote":     _require(f, "code_quote", str),
            "description":    _require(f, "description", str),
            "engine_path":    list(f.get("engine_path") or []),
            "check":          str(f.get("check") or ""),
            "affected_cards": list(f.get("affected_cards") or []),
            "tests":          tests})
    insights = [{"title": _require(i, "title", str),
                 "description": _require(i, "description", str)}
                for i in (d.get("insights") or [])]
    return {"card": str(d.get("card") or ""),
            "card_data_status": str(d.get("card_data_status") or ""),
            "checks": dict(d.get("checks_performed") or {}),
            "findings": findings, "insights": insights,
            "untested_rulings": list(d.get("untested_rulings") or []),
            "is_pass": len(findings) == 0}


def load_test(path: Path) -> dict:
    d = _load_json(path)
    tests = []
    for i, t in enumerate(_require_objects(d, "tests")):
        slug = _require(t, "slug", str)
        status = _require(t, "status", str).lower()
        if status not in VALID_TEST_STATUSES:
            raise StagingError(f"tests[{i}] {slug!r}: status must be one of "
                               f"{sorted(VALID_TEST_STATUSES)}, got {status!r}")
        tests.append({"slug": slug, "status": status,
                      "test_name": str(t.get("test_name") or slug),
                      "assertion_message": str(t.get("assertion_message") or ""),
                      "explanation": str(t.get("explanation") or ""),
                      "blocked_by": t.get("blocked_by") or None})
    return {"test_file": str(d.get("test_file") or ""), "tests": tests}


def load_fix(path: Path) -> dict:
    d = _load_json(path)
    status = _require(d, "status", str).lower()
    if status not in ("fixed", "failed"):
        raise StagingError(f"status must be 'fixed' or 'failed', got {status!r}")
    return {"status": status,
            "files_changed": list(d.get("files_changed") or []),
            "description": str(d.get("description") or "")}


def load_consolidation(path: Path) -> dict:
    d = _load_json(path)
    slug = _require(d, "slug", str)
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]*", slug):
        raise StagingError(f"slug must be lowercase-kebab-case, got {slug!r}")
    tests = []
    for i, t in enumerate(_require_objects(d, "tests")):
        tslug = _require(t, "slug", str)
        if not re.fullmatch(r"[a-z0-9_][a-z0-9_]*", tslug):
            raise StagingError(
                f"tests[{i}]: slug must be snake_case, got {tslug!r}")
        src = t.get("source_ticket")
        if src in (None, "", "(new)"):
            src = None
        tests.append({"slug": tslug, "source_ticket": src,
                      "scenario": _require(t, "scenario", str)})
    if not tests:
        raise StagingError("tests array must contain at least one entry")
    return {"slug": slug, "title": _require(d, "title", str),
            "description": _require(d, "description", str),
            "engine_path": list(d.get("engine_path") or []),
            "tests": tests, "also_closes": list(d.get("also_closes") or [])}
