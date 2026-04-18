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


class SandboxRenderTest(unittest.TestCase):
    """Each profile renders to valid JSON with the expected role rules."""

    def test_unknown_profile_raises(self) -> None:
        """Unknown profile name → ValueError, no silent fallback."""
        with self.assertRaises(ValueError):
            sandbox.render("nonexistent", project_root="/tmp")

    def test_auditor_profile_no_engine_writes(self) -> None:
        """Auditor profile locks out engine, cards, pipeline source writes."""
        rendered = sandbox.render("auditor", project_root="/tmp/repo")
        cfg = json.loads(rendered)
        self.assertTrue(cfg["sandbox"]["enabled"])
        self.assertFalse(cfg["sandbox"]["allowUnsandboxedCommands"])
        denies = cfg["sandbox"]["filesystem"]["denyWrite"]
        self.assertIn("./mtg-engine/**", denies)
        self.assertIn("./cards/**", denies)
        self.assertIn("./pipeline/**", denies)
        # Pipeline source files are denied (engine + commands + tests).
        self.assertIn("./new_pipeline/commands/**", denies)
        self.assertIn("./new_pipeline/sandbox/**", denies)
        self.assertIn("./new_pipeline/tickets/archive/**", denies)
        # Permissions deny mirrors the kernel-level rules for Edit/Write.
        perm_denies = cfg["permissions"]["deny"]
        self.assertIn("Edit(mtg-engine/**)", perm_denies)
        self.assertIn("Write(new_pipeline/commands/**)", perm_denies)
        self.assertIn("Read(new_pipeline/tickets/archive/**)", perm_denies)

    def test_test_writer_default_blocks_engine_src_writes(self) -> None:
        """Default test-writer profile denies `mtg-engine/src/` writes."""
        rendered = sandbox.render("test_writer", project_root="/tmp/repo")
        cfg = json.loads(rendered)
        denies = cfg["sandbox"]["filesystem"]["denyWrite"]
        self.assertIn("./mtg-engine/src/**", denies)
        # Tests dir is NOT denied — that's where the test-writer writes.
        self.assertNotIn("./mtg-engine/tests/**", denies)
        # Staging dir on the main repo is allow-listed (it's outside cwd).
        allows = cfg["sandbox"]["filesystem"]["allowWrite"]
        self.assertIn("/tmp/repo/new_pipeline/staging/", allows)

    def test_test_writer_engine_permits_engine_src_writes(self) -> None:
        """needs_engine_work retry profile allows `mtg-engine/src/` writes."""
        rendered = sandbox.render(
            "test_writer_engine", project_root="/tmp/repo",
        )
        cfg = json.loads(rendered)
        denies = cfg["sandbox"]["filesystem"]["denyWrite"]
        # The whole point: this profile does NOT lock engine src.
        self.assertNotIn("./mtg-engine/src/**", denies)
        perm_denies = cfg["permissions"]["deny"]
        self.assertNotIn("Edit(mtg-engine/src/**)", perm_denies)
        # ...but pipeline source is still locked.
        self.assertIn("./new_pipeline/**", denies)

    def test_fixer_blocks_test_file_writes(self) -> None:
        """Fixer can write engine src but not the test file (ground truth)."""
        rendered = sandbox.render("fixer", project_root="/tmp/repo")
        cfg = json.loads(rendered)
        denies = cfg["sandbox"]["filesystem"]["denyWrite"]
        # Fixer must not modify the test it's being measured against.
        self.assertIn("./mtg-engine/tests/**", denies)
        # ...but engine src IS writable (the whole point of fixer).
        self.assertNotIn("./mtg-engine/src/**", denies)
        perm_denies = cfg["permissions"]["deny"]
        self.assertIn("Edit(mtg-engine/tests/**)", perm_denies)
        self.assertNotIn("Edit(mtg-engine/src/**)", perm_denies)

    def test_substitution_uses_provided_project_root(self) -> None:
        """`${project_root}` placeholders are substituted; none remain raw."""
        rendered = sandbox.render(
            "fixer", project_root="/Users/foo/repo",
        )
        cfg = json.loads(rendered)
        allows = cfg["sandbox"]["filesystem"]["allowWrite"]
        self.assertIn("/Users/foo/repo/new_pipeline/staging/", allows)
        # No unsubstituted placeholders left.
        self.assertNotIn("${", rendered)

    def test_missing_var_raises(self) -> None:
        """Required `${var}` not provided → KeyError, not silent empty sub."""
        with self.assertRaises(KeyError):
            sandbox.render("fixer")

    def test_all_profiles_share_the_security_baseline(self) -> None:
        """sandbox.enabled + allowUnsandboxedCommands=false on every profile.

        These two together are what makes the sandbox non-bypassable.
        Drop either one and the agent can use the dangerouslyDisableSandbox
        escape hatch or skip the sandbox entirely.
        """
        profiles = (
            "auditor", "test_writer", "test_writer_engine", "fixer",
        )
        for profile in profiles:
            with self.subTest(profile=profile):
                cfg = json.loads(
                    sandbox.render(profile, project_root="/tmp/repo")
                )
                self.assertTrue(cfg["sandbox"]["enabled"])
                self.assertFalse(cfg["sandbox"]["allowUnsandboxedCommands"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
