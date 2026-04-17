"""Tests for `validate_test` — cargo-based verifier for agent tests.

A fake `cargo` executable is dropped into a temp bin dir and prepended
to `PATH`, so `validate_test` spawns a real subprocess. The fake's
behavior (exit code, stdout, stderr) is driven by env vars so each
test can script the outcome it wants to exercise.
"""

from __future__ import annotations

import os
import shutil
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline.validate import TestValidation, validate_test  # noqa: E402

_FAKE_CARGO = textwrap.dedent(
    """\
    #!/usr/bin/env python3
    # Scripted cargo stand-in. Reads the outcome from env vars.
    import os
    import sys

    sys.stdout.write(os.environ.get("FAKE_CARGO_STDOUT", ""))
    sys.stderr.write(os.environ.get("FAKE_CARGO_STDERR", ""))
    sys.exit(int(os.environ.get("FAKE_CARGO_EXIT", "0")))
    """
)

_GOOD_TEST_SRC = textwrap.dedent(
    """\
    #[test]
    fn test_thing() {
        assert!(false, "this bug is real");
    }
    """
)


class ValidateTestTest(unittest.TestCase):
    """Pre-cargo checks + every cargo outcome `validate_test` distinguishes."""

    def setUp(self) -> None:
        """Temp worktree + fake cargo on PATH + env vars cleared."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-val-"))
        self.worktree = self.tmp / "wt"
        self.worktree.mkdir()
        (self.worktree / "mtg-engine" / "tests").mkdir(parents=True)
        self.test_file_rel = "mtg-engine/tests/pipeline_bugs_foo.rs"

        bindir = self.tmp / "bin"
        bindir.mkdir()
        fake_cargo = bindir / "cargo"
        fake_cargo.write_text(_FAKE_CARGO)
        fake_cargo.chmod(0o755)

        new_path = f"{bindir}:{os.environ.get('PATH', '')}"
        self._patches = [
            patch.dict(os.environ, {"PATH": new_path}, clear=False),
        ]
        for p in self._patches:
            p.start()
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        for k in (
            "FAKE_CARGO_STDOUT", "FAKE_CARGO_STDERR", "FAKE_CARGO_EXIT",
        ):
            os.environ.pop(k, None)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _write_test(self, contents: str) -> None:
        (self.worktree / self.test_file_rel).write_text(contents)

    def _run(self) -> TestValidation:
        return validate_test(
            self.worktree, self.test_file_rel, "test_thing",
        )

    # ── pre-cargo checks ──────────────────────────────────────────

    def test_missing_test_file_rejects(self) -> None:
        """Test file not on disk → fail without running cargo."""
        result = self._run()
        self.assertFalse(result.ok)
        self.assertIn("does not exist", result.reason)

    def test_banned_phrase_rejects(self) -> None:
        """Placeholder text like TODO / 'would need to' blocks validation."""
        self._write_test(
            "// TODO: fix this later\n"
            "#[test]\n"
            "fn test_thing() { assert!(false); }\n"
        )
        result = self._run()
        self.assertFalse(result.ok)
        self.assertIn("banned placeholder", result.reason)

    def test_no_assertions_rejects(self) -> None:
        """A test file with zero assertions is rejected."""
        self._write_test("#[test]\nfn test_thing() { let _ = 42; }\n")
        result = self._run()
        self.assertFalse(result.ok)
        self.assertIn("no assertions", result.reason)

    # ── cargo outcomes ────────────────────────────────────────────

    def test_cargo_passes_is_false_positive(self) -> None:
        """Cargo exit 0 → the scenario is NOT a real bug."""
        self._write_test(_GOOD_TEST_SRC)
        os.environ["FAKE_CARGO_EXIT"] = "0"
        os.environ["FAKE_CARGO_STDOUT"] = "test result: ok. 1 passed\n"
        result = self._run()
        self.assertFalse(result.ok)
        self.assertIn("false positive", result.reason)

    def test_cargo_fails_with_assertion_is_confirmed_bug(self) -> None:
        """Cargo nonzero + 'assertion failed' in output → real bug."""
        self._write_test(_GOOD_TEST_SRC)
        os.environ["FAKE_CARGO_EXIT"] = "101"
        os.environ["FAKE_CARGO_STDOUT"] = (
            "---- test_thing stdout ----\n"
            "assertion `left == right` failed\n"
            "  left: 20\n right: 21\n"
        )
        result = self._run()
        self.assertTrue(result.ok)
        self.assertIn("assertion", result.reason)

    def test_cargo_panics_without_assertion_is_rejected(self) -> None:
        """Cargo fails but the output is a panic, not an assertion."""
        self._write_test(_GOOD_TEST_SRC)
        os.environ["FAKE_CARGO_EXIT"] = "101"
        os.environ["FAKE_CARGO_STDOUT"] = (
            "---- test_thing stdout ----\n"
            "thread 'main' panicked at 'index out of bounds'\n"
        )
        result = self._run()
        self.assertFalse(result.ok)
        self.assertIn("panic", result.reason)

    def test_cargo_unclear_failure_is_rejected(self) -> None:
        """Nonzero exit without any recognizable failure marker."""
        self._write_test(_GOOD_TEST_SRC)
        os.environ["FAKE_CARGO_EXIT"] = "2"
        os.environ["FAKE_CARGO_STDOUT"] = "linker error\n"
        result = self._run()
        self.assertFalse(result.ok)
        self.assertIn("unclear", result.reason)

    def test_output_tail_is_captured_on_failure(self) -> None:
        """On cargo failure, `output` carries the stdout/stderr tail."""
        self._write_test(_GOOD_TEST_SRC)
        os.environ["FAKE_CARGO_EXIT"] = "101"
        os.environ["FAKE_CARGO_STDOUT"] = (
            "some prelude\nassertion failed: left == right\n"
        )
        result = self._run()
        self.assertIn("assertion failed", result.output)


if __name__ == "__main__":
    unittest.main(verbosity=2)
