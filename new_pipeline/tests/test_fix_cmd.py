"""End-to-end tests for `cmd_fix`.

Spawns real subprocesses for both `claude` (the fixer agent) and
`cargo` (the validator). The real `run_agent_loop`, `validate_fix`,
prompt build, and ticket I/O all run. Only the two external binaries
are swapped for scripted Python stand-ins.

Setup mirrors a post-test-phase ticket: a `tested`-status ticket,
its per-ticket worktree with the test file committed on `fix/<tid>`,
and `tested_sha` pointing at that commit.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import utils, worktree  # noqa: E402
from new_pipeline.commands import fix as fix_mod  # noqa: E402
from new_pipeline.types import Status, Ticket  # noqa: E402

# Fake claude: parse -p <prompt>, find the staging path in the prompt,
# copy an env-supplied payload to it, optionally commit a "fix" file,
# emit a stream-json result.
_FAKE_CLAUDE = textwrap.dedent(
    '''\
    #!/usr/bin/env python3
    import json
    import os
    import re
    import subprocess
    import sys

    argv = sys.argv[1:]
    prompt = argv[argv.index("-p") + 1]

    if os.environ.get("FAKE_AGENT_FAIL"):
        sys.stdout.write(json.dumps({
            "type": "result",
            "usage": {"input_tokens": 1, "output_tokens": 1},
            "num_turns": 1,
            "is_error": True,
            "result": os.environ.get("FAKE_AGENT_FAIL"),
        }) + "\\n")
        sys.exit(0)

    # Staging path (absolute, .json).
    m = re.search(r"(/\\S+?\\.json)\\b", prompt)
    payload_file = os.environ.get("FAKE_AGENT_PAYLOAD_FILE")
    if m and payload_file:
        staging_path = m.group(1)
        os.makedirs(os.path.dirname(staging_path), exist_ok=True)
        open(staging_path, "w").write(open(payload_file).read())

    # Optional: drop a "fix" file and commit it so the diff is non-empty.
    # On retry the file contents may already be in place — commit only
    # if there's something to commit (retry idempotency).
    fix_contents = os.environ.get("FAKE_AGENT_FIX_CONTENTS")
    if fix_contents is not None:
        rel = os.environ.get(
            "FAKE_AGENT_FIX_FILE", "mtg-engine/src/engine.rs",
        )
        abspath = os.path.join(os.getcwd(), rel)
        os.makedirs(os.path.dirname(abspath), exist_ok=True)
        open(abspath, "w").write(fix_contents)
        pending = subprocess.run(
            ["git", "status", "--porcelain"], cwd=os.getcwd(),
            capture_output=True, text=True,
        ).stdout.strip()
        if pending:
            subprocess.run(
                ["git", "add", "-A"], cwd=os.getcwd(), check=True,
            )
            subprocess.run(
                ["git", "commit", "-q", "-m", "fake fix"],
                cwd=os.getcwd(), check=True,
            )

    sys.stdout.write(json.dumps({
        "type": "result",
        "usage": {"input_tokens": 100, "output_tokens": 50},
        "num_turns": 2,
    }) + "\\n")
    '''
)

_FAKE_CARGO = textwrap.dedent(
    """\
    #!/usr/bin/env python3
    import os
    import sys

    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    prefix = f"FAKE_CARGO_{cmd.upper()}_"
    sys.stdout.write(os.environ.get(prefix + "STDOUT", ""))
    sys.stderr.write(os.environ.get(prefix + "STDERR", ""))
    sys.exit(int(os.environ.get(prefix + "EXIT", "0")))
    """
)


def _git(*args: str, cwd: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args],
        cwd=str(cwd),
        check=True,
        capture_output=True,
        text=True,
    )


def _args(
    tickets: str, model: str = "opus", effort: str = "max",
) -> argparse.Namespace:
    return argparse.Namespace(tickets=tickets, model=model, effort=effort)


class FixCommandTest(unittest.TestCase):
    """cmd_fix exercised against a real git repo + scripted claude/cargo."""

    def setUp(self) -> None:
        """Temp git repo, per-ticket worktree, fake binaries on PATH."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-fix-"))
        _git("init", "-q", "-b", "master", cwd=self.tmp)
        _git("config", "user.email", "t@example.com", cwd=self.tmp)
        _git("config", "user.name", "Fix Test", cwd=self.tmp)
        (self.tmp / "README.md").write_text("baseline\n")
        _git("add", "-A", cwd=self.tmp)
        _git("commit", "-q", "-m", "init", cwd=self.tmp)

        tickets = self.tmp / "new_pipeline" / "tickets"
        staging = self.tmp / "new_pipeline" / "staging"
        prompts = self.tmp / "new_pipeline" / "prompts"
        tickets.mkdir(parents=True)
        prompts.mkdir(parents=True)
        # Copy the real fixer prompt in.
        (prompts / "fixer.md").write_text(
            (_THIS.parents[1] / "prompts" / "fixer.md").read_text()
        )
        self.tickets = tickets
        self.prompts = prompts
        self.worktrees = self.tmp / ".worktrees"

        bindir = self.tmp / "bin"
        bindir.mkdir()
        (bindir / "claude").write_text(_FAKE_CLAUDE)
        (bindir / "claude").chmod(0o755)
        (bindir / "cargo").write_text(_FAKE_CARGO)
        (bindir / "cargo").chmod(0o755)

        new_path = f"{bindir}:{os.environ.get('PATH', '')}"
        self._patches = [
            patch.object(utils, "PROJECT_ROOT", self.tmp),
            patch.object(utils, "TICKETS_DIR", tickets),
            patch.object(utils, "ARCHIVE_DIR", tickets / "archive"),
            patch.object(utils, "STAGING_DIR", staging),
            patch.object(utils, "PROMPTS_DIR", prompts),
            patch.object(utils, "WORKTREES_DIR", self.worktrees),
            patch.dict(os.environ, {"PATH": new_path}, clear=False),
        ]
        for p in self._patches:
            p.start()
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        for k in list(os.environ):
            if k.startswith("FAKE_AGENT_") or k.startswith("FAKE_CARGO_"):
                del os.environ[k]
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _setup_tested_ticket(
        self,
        ticket_id: str,
        *,
        test_file_rel: str = "mtg-engine/tests/pipeline_bugs_foo_01.rs",
        test_contents: str = (
            "#[test]\nfn scenario() { assert!(false, \"bug\"); }\n"
        ),
    ) -> str:
        """Create the worktree, commit a test file, return the tested_sha."""
        wt = worktree.ensure(ticket_id)
        test_path = wt / test_file_rel
        test_path.parent.mkdir(parents=True, exist_ok=True)
        test_path.write_text(test_contents)
        _git("add", "-A", cwd=wt)
        _git("commit", "-q", "-m", "tests for fix", cwd=wt)
        tested_sha = _git(
            "rev-parse", "HEAD", cwd=wt,
        ).stdout.strip()
        # Write the ticket file with tested status + phase metadata.
        (self.tickets / f"{ticket_id}.md").write_text(
            textwrap.dedent(
                f"""\
                ---
                id: {ticket_id}
                status: tested
                card: Fake Card
                test_file: {test_file_rel}
                tested_sha: {tested_sha}
                ---

                ## Audit Finding
                A fake finding.

                ## Test Run Results

                - **scenario** — confirmed
                  - test fn: `scenario`
                """
            )
        )
        return tested_sha

    def _set_agent_payload(self, payload: dict) -> None:
        """Point the fake claude at a canned FixReport payload."""
        p = self.tmp / "fix-payload.json"
        p.write_text(json.dumps(payload))
        os.environ["FAKE_AGENT_PAYLOAD_FILE"] = str(p)

    # ── happy path ────────────────────────────────────────────────

    def test_success_marks_ticket_fixed(self) -> None:
        """Agent commits a fix, cargo validates → status → FIXED."""
        self._setup_tested_ticket("foo-01")
        self._set_agent_payload({
            "status": "fixed",
            "description": "Added the missing lifelink branch.",
        })
        os.environ["FAKE_AGENT_FIX_CONTENTS"] = (
            "// fix\npub fn run() { let _ = 42; }\n"
        )
        # cargo check + cargo test both default to exit 0 / empty output.

        fix_mod.cmd_fix(_args("foo-01"))

        t = Ticket.load("foo-01")
        self.assertIs(t.status, Status.FIXED)
        fm = t.frontmatter
        self.assertTrue(fm.fix_run_id.endswith("-foo-01-fix"))
        self.assertEqual(fm.fix_model, "opus")
        self.assertEqual(fm.fix_tokens, 150)
        self.assertEqual(len(fm.fixed_sha), 40)
        self.assertIsNotNone(fm.fixed_at)
        self.assertIsNone(fm.fix_failed_at)
        self.assertIn("## Fix Result", t.body)
        self.assertIn("Added the missing lifelink branch", t.body)

    # ── agent gave up ─────────────────────────────────────────────

    def test_agent_reports_failed_marks_fix_failed(self) -> None:
        """Agent returns `failed` + post-mortem → FIX_FAILED + body note."""
        self._setup_tested_ticket("giveup-01")
        self._set_agent_payload({
            "status": "failed",
            "description": (
                "Tried X, got stuck on Y, would need engine change Z."
            ),
        })
        # No FAKE_AGENT_FIX_CONTENTS → no commit → no diff to validate.
        # cargo isn't called because validator short-circuits on FAILED.

        fix_mod.cmd_fix(_args("giveup-01"))

        t = Ticket.load("giveup-01")
        self.assertIs(t.status, Status.FIX_FAILED)
        fm = t.frontmatter
        self.assertTrue(fm.fix_run_id.endswith("-giveup-01-fix"))
        self.assertIsNotNone(fm.fix_failed_at)
        self.assertEqual(fm.fixed_sha, "")
        self.assertIsNone(fm.fixed_at)
        self.assertIn("## Fix Result", t.body)
        self.assertIn("got stuck on Y", t.body)

    # ── skip / error paths ────────────────────────────────────────

    def test_non_tested_ticket_is_skipped(self) -> None:
        """A ticket whose status isn't `tested` is skipped, not modified."""
        # Create a new-status ticket (no worktree, no metadata needed).
        (self.tickets / "still-new-01.md").write_text(textwrap.dedent(
            """\
            ---
            id: still-new-01
            status: new
            card: Card
            ---

            body
            """
        ))

        fix_mod.cmd_fix(_args("still-new-01"))

        t = Ticket.load("still-new-01")
        self.assertIs(t.status, Status.NEW)
        self.assertEqual(t.frontmatter.fix_run_id, "")

    def test_agent_error_keeps_ticket_tested(self) -> None:
        """Persistent agent error → ticket stays TESTED (retryable)."""
        self._setup_tested_ticket("err-01")
        os.environ["FAKE_AGENT_FAIL"] = "agent blew up"

        fix_mod.cmd_fix(_args("err-01"))

        t = Ticket.load("err-01")
        self.assertIs(t.status, Status.TESTED)
        self.assertEqual(t.frontmatter.fix_run_id, "")

    def test_cargo_check_warning_keeps_ticket_tested(self) -> None:
        """Agent claims `fixed` but cargo warns → retries exhaust → TESTED."""
        self._setup_tested_ticket("warn-01")
        self._set_agent_payload({
            "status": "fixed",
            "description": "stubbed fix with unused import",
        })
        os.environ["FAKE_AGENT_FIX_CONTENTS"] = (
            "// fix\npub fn run() {}\n"
        )
        os.environ["FAKE_CARGO_CHECK_STDOUT"] = (
            "warning[unused_imports]: unused import `Foo`\n"
        )

        fix_mod.cmd_fix(_args("warn-01"))

        t = Ticket.load("warn-01")
        self.assertIs(t.status, Status.TESTED)
        self.assertEqual(t.frontmatter.fix_run_id, "")

    # ── arg validation ────────────────────────────────────────────

    def test_empty_tickets_raises(self) -> None:
        """`--tickets ' , '` → ValueError."""
        with self.assertRaises(ValueError):
            fix_mod.cmd_fix(_args(" , "))


if __name__ == "__main__":
    unittest.main(verbosity=2)
