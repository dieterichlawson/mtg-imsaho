"""Parse a fixer staging JSON file.

`FixReport.load` is the consumer for the agent's output, tested the
same way `AuditReport.load` and `TestReport.load` are tested: hand-
author a JSON file, parse it, exercise every rejection branch.
"""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline.types import (  # noqa: E402
    FixReport,
    FixStatus,
    StagingError,
)


def _write_json(path: Path, payload: dict) -> Path:
    path.write_text(json.dumps(payload))
    return path


class FixReportTest(unittest.TestCase):
    """Parse happy + malformed fixer staging files."""

    def setUp(self) -> None:
        """Give each test a private tmp dir for the staging file."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-fr-"))
        self.staging = self.tmp / "report.json"
        self.addCleanup(lambda: shutil.rmtree(self.tmp, ignore_errors=True))

    # ── happy path ─────────────────────────────────────────────────

    def test_load_parses_fixed_status(self) -> None:
        """A `fixed` report round-trips through parsing."""
        _write_json(self.staging, {
            "status": "fixed",
            "description": (
                "Fixed the lifelink trigger to fire on first-strike."
            ),
        })
        report = FixReport.load(self.staging)
        self.assertIs(report.status, FixStatus.FIXED)
        self.assertIn("lifelink", report.description)

    def test_load_parses_failed_status(self) -> None:
        """A `failed` report with a post-mortem round-trips."""
        _write_json(self.staging, {
            "status": "failed",
            "description": "Tried X, got stuck on Y, would need Z.",
        })
        report = FixReport.load(self.staging)
        self.assertIs(report.status, FixStatus.FAILED)
        self.assertIn("stuck on Y", report.description)

    def test_status_is_case_insensitive(self) -> None:
        """`"Fixed"` or `"FIXED"` both lower-case to `fixed`."""
        _write_json(self.staging, {
            "status": "FIXED", "description": "all good",
        })
        self.assertIs(FixReport.load(self.staging).status, FixStatus.FIXED)

    def test_to_result_section_includes_status_and_description(self) -> None:
        """`to_result_section` renders the status marker + description."""
        report = FixReport(
            status=FixStatus.FIXED,
            description="Added the missing lifelink trigger.",
        )
        section = report.to_result_section()
        self.assertIn("## Fix Result", section)
        self.assertIn("**Status:** fixed", section)
        self.assertIn("Added the missing lifelink trigger.", section)

    # ── malformed input → StagingError ────────────────────────────

    def test_missing_status_raises(self) -> None:
        """Top-level `status` is required."""
        _write_json(self.staging, {"description": "hello"})
        with self.assertRaises(StagingError):
            FixReport.load(self.staging)

    def test_invalid_status_raises(self) -> None:
        """A status outside `fixed`/`failed` is rejected."""
        _write_json(self.staging, {
            "status": "maybe", "description": "hello",
        })
        with self.assertRaises(StagingError) as ctx:
            FixReport.load(self.staging)
        self.assertIn("must be one of", str(ctx.exception))

    def test_missing_description_raises(self) -> None:
        """`description` is required on both statuses."""
        _write_json(self.staging, {"status": "fixed"})
        with self.assertRaises(StagingError):
            FixReport.load(self.staging)

    def test_empty_description_raises(self) -> None:
        """An empty/whitespace description is rejected too."""
        _write_json(self.staging, {
            "status": "failed", "description": "   ",
        })
        with self.assertRaises(StagingError):
            FixReport.load(self.staging)

    def test_wrong_status_type_raises(self) -> None:
        """`status` of the wrong JSON type is rejected."""
        _write_json(self.staging, {
            "status": 42, "description": "hello",
        })
        with self.assertRaises(StagingError):
            FixReport.load(self.staging)


if __name__ == "__main__":
    unittest.main(verbosity=2)
