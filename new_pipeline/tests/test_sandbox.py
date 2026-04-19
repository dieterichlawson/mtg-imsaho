"""Tests for `new_pipeline.sandbox.render`.

Verifies that each profile loads, parses as JSON, substitutes
placeholders, and ends up with the expected per-role rules. Catches
typos in the JSON templates and missing/extra `${var}` placeholders
before they break a real agent run.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import sandbox  # noqa: E402

_VARS = {
    "project_root": "/tmp/repo",
    "home": "/Users/test",
    "test_file": "mtg-engine/tests/pipeline_bugs_foo_01.rs",
}


class SandboxRenderTest(unittest.TestCase):
    """Each profile renders to valid JSON with the expected role rules."""

    def test_unknown_profile_raises(self) -> None:
        """Unknown profile name → ValueError, no silent fallback."""
        with self.assertRaises(ValueError):
            sandbox.render("nonexistent", **_VARS)

    def test_auditor_writes_only_staging(self) -> None:
        """Auditor allowWrite is the minimal set: staging + system tmp.

        The agent only emits a report JSON to staging. Ticket files
        are created by Python (`AuditReport.mint_tickets()`) AFTER
        the agent run finishes — that runs in the main process,
        outside srt's sandbox. So the agent itself never needs
        ticket-dir write access.

        ~/.claude is also absent: claude is invoked with
        `--no-session-persistence` and shouldn't need to write its
        own metadata dir during a one-shot `-p` run.
        """
        cfg = json.loads(sandbox.render("auditor", **_VARS))
        allows = cfg["filesystem"]["allowWrite"]
        self.assertIn("./new_pipeline/staging", allows)
        self.assertIn("/tmp", allows)
        # Things that should NOT be writable (kernel rejects).
        self.assertNotIn("./new_pipeline/tickets", allows)
        self.assertNotIn("/Users/test/.claude", allows)
        self.assertNotIn("./mtg-engine", allows)
        self.assertNotIn("./cards", allows)
        self.assertNotIn(".", allows)
        self.assertNotIn("./", allows)

    def test_test_writer_locks_to_single_test_file(self) -> None:
        """test_writer profile: only the specific test file is writable.

        Locked to `./<test_file>` rather than the whole tests dir, so
        the agent can't write a stray test file or overwrite an
        unrelated existing test.
        """
        cfg = json.loads(sandbox.render("test_writer", **_VARS))
        allows = cfg["filesystem"]["allowWrite"]
        self.assertIn("./mtg-engine/tests/pipeline_bugs_foo_01.rs", allows)
        # NOT the whole tests dir — single file only.
        self.assertNotIn("./mtg-engine/tests", allows)
        # Engine src locked.
        self.assertNotIn("./mtg-engine/src", allows)
        # Cargo writes target/ on the main repo (shared via
        # CARGO_TARGET_DIR). The worktree's local ./target is NOT in
        # the allow list — cargo never writes there.
        self.assertIn("/tmp/repo/target", allows)
        self.assertNotIn("./target", allows)
        # Git commits to worktree's .git/.
        self.assertIn("./.git", allows)
        # Staging on the main repo is allow-listed (outside cwd).
        self.assertIn("/tmp/repo/new_pipeline/staging", allows)
        # Tightened: ~/.claude and ~/.cargo should NOT be in allowWrite.
        self.assertNotIn("/Users/test/.claude", allows)
        self.assertNotIn("/Users/test/.cargo", allows)

    def test_test_writer_engine_adds_engine_src_and_cards(self) -> None:
        """needs_engine_work retry profile: engine src + cards added."""
        cfg = json.loads(sandbox.render("test_writer_engine", **_VARS))
        allows = cfg["filesystem"]["allowWrite"]
        # The whole point of the engine variant.
        self.assertIn("./mtg-engine/src", allows)
        self.assertIn("./cards", allows)
        # Test writer's own single-file write still allowed.
        self.assertIn("./mtg-engine/tests/pipeline_bugs_foo_01.rs", allows)
        self.assertNotIn("./mtg-engine/tests", allows)

    def test_fixer_omits_test_dir(self) -> None:
        """Fixer can write engine src + cards, NOT the test file."""
        cfg = json.loads(sandbox.render("fixer", **_VARS))
        allows = cfg["filesystem"]["allowWrite"]
        self.assertIn("./mtg-engine/src", allows)
        self.assertIn("./cards", allows)
        # Tests are the ground truth — fixer must not touch them.
        self.assertNotIn("./mtg-engine/tests", allows)

    def test_substitution_uses_provided_vars(self) -> None:
        """`${project_root}` and `${home}` are substituted; none remain."""
        rendered = sandbox.render(
            "fixer", project_root="/Users/foo/repo", home="/Users/foo",
        )
        cfg = json.loads(rendered)
        allows = cfg["filesystem"]["allowWrite"]
        self.assertIn("/Users/foo/repo/new_pipeline/staging", allows)
        # ${home} is substituted in denyRead (sensitive home dirs).
        deny_reads = cfg["filesystem"]["denyRead"]
        self.assertIn("/Users/foo/.ssh", deny_reads)
        self.assertNotIn("${", rendered)

    def test_missing_var_raises(self) -> None:
        """Required `${var}` not provided → KeyError, not silent empty sub."""
        with self.assertRaises(KeyError):
            sandbox.render("fixer", project_root="/tmp")  # missing `home`

    def test_scryfall_in_network_allowlist(self) -> None:
        """Scryfall is allowed for oracle text lookups."""
        for profile in (
            "auditor", "test_writer", "test_writer_engine", "fixer",
        ):
            with self.subTest(profile=profile):
                cfg = json.loads(sandbox.render(profile, **_VARS))
                domains = cfg["network"]["allowedDomains"]
                self.assertIn("*.scryfall.com", domains)

    def test_sensitive_home_dirs_denied_for_reads(self) -> None:
        """All profiles deny reads of ~/.ssh, ~/.aws, ~/.gnupg.

        The agent never legitimately needs these; defense in depth
        against a prompt-injected exfil attempt.
        """
        for profile in (
            "auditor", "test_writer", "test_writer_engine", "fixer",
        ):
            with self.subTest(profile=profile):
                cfg = json.loads(sandbox.render(profile, **_VARS))
                deny_reads = cfg["filesystem"]["denyRead"]
                self.assertIn("/Users/test/.ssh", deny_reads)
                self.assertIn("/Users/test/.aws", deny_reads)
                self.assertIn("/Users/test/.gnupg", deny_reads)

    def test_all_profiles_have_complete_srt_schema(self) -> None:
        """Srt requires every config key present (even if empty array)."""
        for profile in (
            "auditor", "test_writer", "test_writer_engine", "fixer",
        ):
            with self.subTest(profile=profile):
                cfg = json.loads(sandbox.render(profile, **_VARS))
                self.assertIn("allowedDomains", cfg["network"])
                self.assertIn("deniedDomains", cfg["network"])
                for key in (
                    "allowWrite", "denyWrite", "allowRead", "denyRead",
                ):
                    self.assertIn(key, cfg["filesystem"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
