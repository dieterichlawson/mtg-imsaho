"""Parse an audit-report JSON file and mint tickets from it.

Exercises `AuditReport.load`, `Finding.to_ticket_body`,
`Ticket.allocate_id`, `Ticket.create`, and `AuditReport.mint_tickets`
end-to-end against a real temp workspace — no agent, no worktrees.
"""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import utils  # noqa: E402
from new_pipeline.types import (  # noqa: E402
    AuditReport,
    Finding,
    FindingTest,
    StagingError,
    Status,
    Ticket,
    TicketError,
)


def _write_json(path: Path, payload: dict) -> Path:
    path.write_text(json.dumps(payload, indent=2))
    return path


def _good_report(**overrides) -> dict:
    base = {
        "card": "Olivia Voldaren",
        "findings": [
            {
                "oracle_quote": "Whenever Olivia deals damage ...",
                "code_quote": "if !source.has_ability(...)",
                "description": "Lifelink trigger skipped on first-strike.",
                "engine_path": "combat/lifelink.rs:42",
                "tests": [
                    {
                        "slug": "test_lifelink_first_strike",
                        "scenario": "Attack with first-strike into blocker.",
                    },
                ],
            },
        ],
    }
    base.update(overrides)
    return base


class AuditIngestTest(unittest.TestCase):
    """End-to-end: JSON report → parsed → tickets on disk."""

    def setUp(self) -> None:
        """Point utils at a fresh temp workspace for each test."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-ingest-"))
        tickets = self.tmp / "tickets"
        archive = tickets / "archive"
        tickets.mkdir()
        self._patches = [
            patch.object(utils, "TICKETS_DIR", tickets),
            patch.object(utils, "ARCHIVE_DIR", archive),
        ]
        for p in self._patches:
            p.start()
        self.staging = self.tmp / "report.json"
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)

    # ── happy path ─────────────────────────────────────────────────

    def test_load_parses_optional_audit_metadata(self) -> None:
        """checks_performed and insights both parse cleanly."""
        payload = _good_report(
            checks_performed={
                "8a": "done — no runtime fields touched",
                "8b": "n/a — no triggered abilities",
            },
            insights=[
                "## A new pattern\n\nEngine omits X check.",
            ],
        )
        _write_json(self.staging, payload)
        report = AuditReport.load(self.staging)
        self.assertEqual(report.checks_performed["8a"],
                         "done — no runtime fields touched")
        self.assertEqual(len(report.insights), 1)
        self.assertIn("A new pattern", report.insights[0])

    def test_load_defaults_optional_metadata_to_empty(self) -> None:
        """Reports without the new fields still parse — defaults are empty."""
        _write_json(self.staging, _good_report())
        report = AuditReport.load(self.staging)
        self.assertEqual(report.checks_performed, {})
        self.assertEqual(report.insights, [])

    def test_insights_must_be_strings(self) -> None:
        """Non-string entry in insights → StagingError."""
        payload = _good_report(insights=["valid", 42, "also valid"])
        _write_json(self.staging, payload)
        with self.assertRaises(StagingError):
            AuditReport.load(self.staging)

    def test_load_parses_report(self) -> None:
        """`AuditReport.load` returns a typed dataclass tree."""
        _write_json(self.staging, _good_report())
        report = AuditReport.load(self.staging)
        self.assertEqual(report.card, "Olivia Voldaren")
        self.assertEqual(len(report.findings), 1)
        f = report.findings[0]
        self.assertIsInstance(f, Finding)
        self.assertEqual(
            f.description, "Lifelink trigger skipped on first-strike.",
        )
        self.assertEqual(f.engine_path, "combat/lifelink.rs:42")
        self.assertEqual(len(f.tests), 1)
        self.assertIsInstance(f.tests[0], FindingTest)

    def test_mint_tickets_creates_one_per_finding(self) -> None:
        """Every finding in the report becomes a ticket on disk."""
        report_payload = _good_report()
        report_payload["findings"].append(
            {
                "oracle_quote": "...",
                "code_quote": "...",
                "description": "Second bug.",
            }
        )
        _write_json(self.staging, report_payload)

        report = AuditReport.load(self.staging)
        created = report.mint_tickets()

        self.assertEqual(len(created), 2)
        self.assertEqual(
            [t.id for t in created],
            ["olivia_voldaren-01", "olivia_voldaren-02"],
        )
        for t in created:
            self.assertIs(t.status, Status.NEW)
            self.assertEqual(t.frontmatter.card, "Olivia Voldaren")
            self.assertTrue(
                (utils.TICKETS_DIR / f"{t.id}.md").exists()
            )

    def test_minted_body_contains_finding_details(self) -> None:
        """The rendered body includes oracle quote, code quote, description."""
        _write_json(self.staging, _good_report())
        created = AuditReport.load(self.staging).mint_tickets()
        body = created[0].body
        self.assertIn("Whenever Olivia deals damage ...", body)
        self.assertIn("if !source.has_ability", body)
        self.assertIn("Lifelink trigger skipped", body)
        self.assertIn("### test_lifelink_first_strike", body)
        self.assertIn("Attack with first-strike into blocker.", body)

    def test_allocate_id_skips_archived(self) -> None:
        """Allocating `{stem}-NN` skips ids already in active + archive."""
        # Seed: olivia_voldaren-01 archived, -02 active.
        (utils.ARCHIVE_DIR).mkdir(parents=True, exist_ok=True)
        (utils.ARCHIVE_DIR / "olivia_voldaren-01.md").write_text(
            "---\nid: olivia_voldaren-01\nstatus: closed\n---\n\nold"
        )
        (utils.TICKETS_DIR / "olivia_voldaren-02.md").write_text(
            "---\nid: olivia_voldaren-02\nstatus: new\n---\n\nolder"
        )
        self.assertEqual(
            Ticket.allocate_id("olivia_voldaren"),
            "olivia_voldaren-03",
        )

    def test_create_rejects_duplicate(self) -> None:
        """Ticket.create refuses an id that already exists."""
        Ticket.create(
            "foo-01", status=Status.NEW, card="Foo", body="body",
        )
        with self.assertRaises(TicketError):
            Ticket.create(
                "foo-01", status=Status.NEW, card="Foo", body="body",
            )

    def test_fallback_test_slug_when_findings_omit_tests(self) -> None:
        """Findings without a `tests` array get a default slug."""
        payload = _good_report()
        payload["findings"][0].pop("tests")
        _write_json(self.staging, payload)

        created = AuditReport.load(self.staging).mint_tickets()
        self.assertIn("### test_olivia_voldaren_01", created[0].body)

    # ── malformed input → StagingError ────────────────────────────

    def test_missing_card_raises(self) -> None:
        """A report missing the required top-level `card` field is rejected."""
        payload = _good_report()
        payload.pop("card")
        _write_json(self.staging, payload)
        with self.assertRaises(StagingError):
            AuditReport.load(self.staging)

    def test_missing_findings_raises(self) -> None:
        """A report missing the required `findings` array is rejected."""
        payload = _good_report()
        payload.pop("findings")
        _write_json(self.staging, payload)
        with self.assertRaises(StagingError):
            AuditReport.load(self.staging)

    def test_finding_missing_required_field_raises(self) -> None:
        """A finding missing a required string field is rejected."""
        payload = _good_report()
        payload["findings"][0].pop("description")
        _write_json(self.staging, payload)
        with self.assertRaises(StagingError):
            AuditReport.load(self.staging)

    def test_wrong_type_raises(self) -> None:
        """A field of the wrong JSON type is rejected with a typed error."""
        payload = _good_report()
        payload["findings"] = "not-a-list"
        _write_json(self.staging, payload)
        with self.assertRaises(StagingError):
            AuditReport.load(self.staging)

    def test_wrong_type_optional_str_raises(self) -> None:
        """An optional string field with the wrong type is rejected."""
        payload = _good_report()
        payload["findings"][0]["engine_path"] = ["list", "instead"]
        _write_json(self.staging, payload)
        with self.assertRaises(StagingError):
            AuditReport.load(self.staging)

    def test_invalid_json_raises(self) -> None:
        """Non-JSON content is rejected with StagingError (not a json error)."""
        self.staging.write_text("{ not valid json }")
        with self.assertRaises(StagingError):
            AuditReport.load(self.staging)


if __name__ == "__main__":
    unittest.main(verbosity=2)
