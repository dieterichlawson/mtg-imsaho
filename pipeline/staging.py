"""Staging JSON — the agent → Python wire format.

Each pipeline stage has its own staging shape. Every shape is a
dataclass with a `load(path)` classmethod that parses + validates.
Every dataclass field corresponds to a top-level JSON key.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from pipeline.utils import parse_slug


class StagingError(ValueError):
    """Raised when a staging JSON file's shape is invalid."""


# ── Low-level parsing helpers ───────────────────────────────────────


def _load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as e:
        raise StagingError(f"{path.name}: invalid JSON — {e}") from None


def _require(d: dict, key: str, kind: type) -> Any:
    if key not in d:
        raise StagingError(f"missing required field: {key!r}")
    v = d[key]
    if not isinstance(v, kind):
        raise StagingError(
            f"field {key!r}: expected {kind.__name__}, got {type(v).__name__}"
        )
    return v


def _require_objects(d: dict, key: str) -> list[dict]:
    arr = _require(d, key, list)
    for i, x in enumerate(arr):
        if not isinstance(x, dict):
            raise StagingError(
                f"{key}[{i}]: expected object, got {type(x).__name__}"
            )
    return arr


# ── Per-test statuses (test-writer report) ─────────────────────────

TEST_CONFIRMED = "confirmed"
TEST_REJECTED = "rejected"
TEST_BLOCKED = "blocked"
VALID_TEST_STATUSES = frozenset({TEST_CONFIRMED, TEST_REJECTED, TEST_BLOCKED})


# ── Auditor ────────────────────────────────────────────────────────


@dataclass
class FindingTest:
    """A single test slug + scenario the auditor wants the test-writer.

    to produce for this finding.
    """

    slug: str
    scenario: str


@dataclass
class Finding:
    """One bug identified by the auditor."""

    oracle_quote: str
    code_quote: str
    description: str
    engine_path: list[str] = field(default_factory=list)
    check: str = ""
    affected_cards: list[str] = field(default_factory=list)
    tests: list[FindingTest] = field(default_factory=list)


@dataclass
class Insight:
    """A generalizable pattern the auditor discovered."""

    title: str
    description: str


@dataclass
class AuditReport:
    """The full staging output from one auditor run."""

    card: str
    card_data_status: str
    checks: dict[str, str]
    findings: list[Finding]
    insights: list[Insight]
    untested_rulings: list[str]

    @property
    def is_pass(self) -> bool:
        """True when the auditor found no bugs."""
        return not self.findings

    @classmethod
    def load(cls, path: Path) -> AuditReport:
        """Parse and validate an auditor staging file."""
        d = _load_json(path)
        findings = [_parse_finding(f) for f in _require_objects(d, "findings")]
        insights = [
            Insight(
                title=_require(i, "title", str),
                description=_require(i, "description", str),
            )
            for i in (d.get("insights") or [])
        ]
        return cls(
            card=str(d.get("card") or ""),
            card_data_status=str(d.get("card_data_status") or ""),
            checks=dict(d.get("checks_performed") or {}),
            findings=findings,
            insights=insights,
            untested_rulings=list(d.get("untested_rulings") or []),
        )


def _parse_finding(f: dict) -> Finding:
    tests = [
        FindingTest(
            slug=_require(t, "slug", str), scenario=_require(t, "scenario", str)
        )
        for t in (f.get("tests") or [])
    ]
    return Finding(
        oracle_quote=_require(f, "oracle_quote", str),
        code_quote=_require(f, "code_quote", str),
        description=_require(f, "description", str),
        engine_path=list(f.get("engine_path") or []),
        check=str(f.get("check") or ""),
        affected_cards=list(f.get("affected_cards") or []),
        tests=tests,
    )


# ── Test-writer ────────────────────────────────────────────────────


@dataclass
class TestResult:
    """What the test-writer reported for one slug in `## Tests`."""

    slug: str
    status: str  # one of VALID_TEST_STATUSES
    test_name: str = ""
    assertion_message: str = ""
    explanation: str = ""
    blocked_by: str | None = None


@dataclass
class TestReport:
    """Staging output from one test-writer run."""

    test_file: str
    tests: list[TestResult]

    @classmethod
    def load(cls, path: Path) -> TestReport:
        """Parse and validate a test-writer staging file."""
        d = _load_json(path)
        tests = []
        for i, t in enumerate(_require_objects(d, "tests")):
            slug = _require(t, "slug", str)
            status = _require(t, "status", str).lower()
            if status not in VALID_TEST_STATUSES:
                raise StagingError(
                    f"tests[{i}] {slug!r}: status must be one of "
                    f"{sorted(VALID_TEST_STATUSES)}, got {status!r}"
                )
            tests.append(
                TestResult(
                    slug=slug,
                    status=status,
                    test_name=str(t.get("test_name") or slug),
                    assertion_message=str(t.get("assertion_message") or ""),
                    explanation=str(t.get("explanation") or ""),
                    blocked_by=t.get("blocked_by") or None,
                )
            )
        return cls(test_file=str(d.get("test_file") or ""), tests=tests)


# ── Fixer ──────────────────────────────────────────────────────────


@dataclass
class FixReport:
    """Staging output from one fixer run."""

    status: str  # "fixed" | "failed"
    files_changed: list[str]
    description: str

    @classmethod
    def load(cls, path: Path) -> FixReport:
        """Parse and validate a fixer staging file."""
        d = _load_json(path)
        status = _require(d, "status", str).lower()
        if status not in ("fixed", "failed"):
            raise StagingError(
                f"status must be 'fixed' or 'failed', got {status!r}"
            )
        return cls(
            status=status,
            files_changed=list(d.get("files_changed") or []),
            description=str(d.get("description") or ""),
        )


# ── Dedup consolidation proposal ───────────────────────────────────


@dataclass
class ProposedTest:
    """A test the dedup agent proposes for a consolidated parent.

    `source_ticket` is always set — every test must come from exactly
    one of the tickets being merged. The dedup agent may not invent
    new tests with no source.
    """

    slug: str
    source_ticket: str
    scenario: str


@dataclass
class ConsolidationProposal:
    """The dedup agent's proposal to merge N tickets into one parent."""

    slug: str
    title: str
    description: str
    engine_path: list[str]
    tests: list[ProposedTest]
    also_closes: list[str]

    @classmethod
    def load(cls, path: Path) -> ConsolidationProposal:
        """Parse and validate a consolidation staging file."""
        d = _load_json(path)
        slug = parse_slug(_require(d, "slug", str), kind="kebab")
        tests = [
            _parse_proposed_test(i, t)
            for i, t in enumerate(_require_objects(d, "tests"))
        ]
        if not tests:
            raise StagingError("tests array must contain at least one entry")
        return cls(
            slug=slug,
            title=_require(d, "title", str),
            description=_require(d, "description", str),
            engine_path=list(d.get("engine_path") or []),
            tests=tests,
            also_closes=list(d.get("also_closes") or []),
        )


def _parse_proposed_test(i: int, t: dict) -> ProposedTest:
    slug = parse_slug(_require(t, "slug", str), kind="snake")
    src = t.get("source_ticket")
    if src in (None, "", "(new)"):
        raise StagingError(
            f"tests[{i}] {slug!r}: dedup proposals must list a "
            f"`source_ticket` for every test — agents cannot invent "
            f"new tests, only combine existing ones from the merged set"
        )
    return ProposedTest(
        slug=slug, source_ticket=str(src), scenario=_require(t, "scenario", str)
    )
