"""Parse a test-writer staging JSON file.

`TestReport.load` is the consumer for the agent-written JSON — tested
the same way `AuditReport.load` is tested in `test_audit_ingest.py`:
hand-author a JSON file, parse it, exercise every rejection branch.
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
    TEST_BLOCKED,
    TEST_CONFIRMED,
    TEST_REJECTED,
    StagingError,
    TestReport,
    TestResult,
)


def _write_json(path: Path, payload: dict) -> Path:
    path.write_text(json.dumps(payload))
    return path


def _good_report() -> dict:
    return {
        "test_file": "mtg-engine/tests/pipeline_bugs_olivia_voldaren_01.rs",
        "tests": [
            {
                "slug": "test_lifelink_first_strike",
                "status": "confirmed",
                "test_name": "test_lifelink_first_strike",
                "assertion_message": "expected life == 21, got 20",
                "explanation": "lifelink didn't trigger on first-strike damage",
            },
            {
                "slug": "test_deathtouch_lethal",
                "status": "rejected",
                "explanation": "passes against current code",
            },
            {
                "slug": "test_blocked_by_engine",
                "status": "blocked",
                "blocked_by": "need pub fn on combat::",
            },
        ],
    }


class TestReportTest(unittest.TestCase):
    """Parse happy + malformed test-writer staging files."""

    def setUp(self) -> None:
        """Give each test a private tmp dir for the staging file."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-tr-"))
        self.staging = self.tmp / "report.json"
        self.addCleanup(lambda: shutil.rmtree(self.tmp, ignore_errors=True))

    # ── happy path ─────────────────────────────────────────────────

    def test_load_parses_every_status_flavor(self) -> None:
        """Confirmed / rejected / blocked all survive the round-trip."""
        _write_json(self.staging, _good_report())
        report = TestReport.load(self.staging)

        self.assertEqual(
            report.test_file,
            "mtg-engine/tests/pipeline_bugs_olivia_voldaren_01.rs",
        )
        self.assertEqual(len(report.tests), 3)
        statuses = [t.status for t in report.tests]
        self.assertEqual(
            statuses, [TEST_CONFIRMED, TEST_REJECTED, TEST_BLOCKED],
        )

    def test_test_name_defaults_to_slug_when_absent(self) -> None:
        """Test-name is a convenience alias of slug when the agent omits it."""
        _write_json(self.staging, {
            "test_file": "a/b.rs",
            "tests": [
                {"slug": "scenario_foo", "status": "confirmed"},
            ],
        })
        report = TestReport.load(self.staging)
        self.assertEqual(report.tests[0].test_name, "scenario_foo")

    def test_blocked_by_roundtrips(self) -> None:
        """`blocked_by: null` parses to None, a string parses verbatim."""
        _write_json(self.staging, {
            "test_file": "a/b.rs",
            "tests": [
                {"slug": "x", "status": "blocked", "blocked_by": "need X"},
                {"slug": "y", "status": "blocked", "blocked_by": None},
            ],
        })
        report = TestReport.load(self.staging)
        self.assertEqual(report.tests[0].blocked_by, "need X")
        self.assertIsNone(report.tests[1].blocked_by)

    def test_status_is_case_insensitive(self) -> None:
        """`"Confirmed"` or `"CONFIRMED"` both lower-case to `confirmed`."""
        _write_json(self.staging, {
            "test_file": "a/b.rs",
            "tests": [{"slug": "x", "status": "CONFIRMED"}],
        })
        report = TestReport.load(self.staging)
        self.assertEqual(report.tests[0].status, TEST_CONFIRMED)

    # ── malformed input → StagingError ────────────────────────────

    def test_missing_test_file_raises(self) -> None:
        """Top-level `test_file` is required."""
        payload = _good_report()
        payload.pop("test_file")
        _write_json(self.staging, payload)
        with self.assertRaises(StagingError):
            TestReport.load(self.staging)

    def test_missing_tests_raises(self) -> None:
        """Top-level `tests` array is required."""
        payload = _good_report()
        payload.pop("tests")
        _write_json(self.staging, payload)
        with self.assertRaises(StagingError):
            TestReport.load(self.staging)

    def test_invalid_status_raises(self) -> None:
        """A status outside the `confirmed/rejected/blocked` set is rejected."""
        _write_json(self.staging, {
            "test_file": "a/b.rs",
            "tests": [{"slug": "x", "status": "maybe"}],
        })
        with self.assertRaises(StagingError) as ctx:
            TestReport.load(self.staging)
        self.assertIn("must be one of", str(ctx.exception))

    def test_blocked_by_wrong_type_raises(self) -> None:
        """`blocked_by` that's neither null nor a string is rejected."""
        _write_json(self.staging, {
            "test_file": "a/b.rs",
            "tests": [
                {"slug": "x", "status": "blocked", "blocked_by": [42]},
            ],
        })
        with self.assertRaises(StagingError):
            TestReport.load(self.staging)

    def test_missing_slug_raises(self) -> None:
        """Every test must have a slug."""
        _write_json(self.staging, {
            "test_file": "a/b.rs",
            "tests": [{"status": "confirmed"}],
        })
        with self.assertRaises(StagingError):
            TestReport.load(self.staging)


class TestResultDirectTest(unittest.TestCase):
    """Cover `TestResult.from_dict` directly for completeness."""

    def test_from_dict_preserves_index_in_error_message(self) -> None:
        """The `i` arg surfaces in error messages for easy debugging."""
        with self.assertRaises(StagingError) as ctx:
            TestResult.from_dict(7, {"slug": "x", "status": "nope"})
        self.assertIn("tests[7]", str(ctx.exception))


if __name__ == "__main__":
    unittest.main(verbosity=2)
