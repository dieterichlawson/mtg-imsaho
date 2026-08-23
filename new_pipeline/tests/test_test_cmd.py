"""End-to-end tests for `cmd_test`.

Spawns real subprocesses for both `claude` (the agent) and `cargo`
(the validator). Each fake is a small Python script dropped into a
temp bin dir that's prepended to `$PATH`; their behavior is driven by
env vars so each test scripts a specific outcome. The rest of the
pipeline — `run_agent_loop`, `worktree.ensure`, `validate_test`,
`TestReport.load`, ticket I/O — runs for real.
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

from new_pipeline import oracle, utils  # noqa: E402
from new_pipeline.commands import test as test_mod  # noqa: E402
from new_pipeline.types import Status, Ticket  # noqa: E402

_FAKE_CLAUDE = textwrap.dedent(
    '''\
    #!/usr/bin/env python3
    """Scripted stand-in for the claude CLI used by test-writer tests.

    Reads -p <prompt>, extracts the staging-file and test-file paths,
    copies env-pointed payloads to them, and emits a stream-json result.
    """
    import json
    import os
    import re
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

    m_json = re.search(r"(/\\S+?\\.json)\\b", prompt)
    m_rs = re.search(r"(mtg-engine/tests/\\S+?\\.rs)\\b", prompt)

    payload_file = os.environ.get("FAKE_AGENT_PAYLOAD_FILE")
    test_src_file = os.environ.get("FAKE_AGENT_TEST_SRC_FILE")

    if m_json and payload_file:
        staging_path = m_json.group(1)
        os.makedirs(os.path.dirname(staging_path), exist_ok=True)
        open(staging_path, "w").write(open(payload_file).read())

    if m_rs and test_src_file:
        rs_path = m_rs.group(1)  # relative to cwd, which is the worktree
        os.makedirs(os.path.dirname(rs_path), exist_ok=True)
        open(rs_path, "w").write(open(test_src_file).read())

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
    # Scripted cargo stand-in. Outcome driven by env vars.
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
    fn test_my_scenario() {
        assert!(false, "this bug is real");
    }
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


def _write_ticket_file(
    tickets_dir: Path,
    ticket_id: str,
    *,
    status: str = "new",
    card: str = "Fake Card",
    slug: str = "test_my_scenario",
) -> None:
    """Write a minimal ticket file with a `## Tests` section to disk."""
    body = textwrap.dedent(
        f"""\
        ---
        id: {ticket_id}
        status: {status}
        card: {card}
        ---

        ## Audit Finding
        A fake finding.

        ## Tests

        ### {slug}
        Scenario: exercise the bug.
        """
    )
    path = tickets_dir / f"{ticket_id}.md"
    path.write_text(body)


class TestCommandTest(unittest.TestCase):
    """cmd_test exercised against a real git repo + scripted claude/cargo."""

    def setUp(self) -> None:
        """Temp git repo as PROJECT_ROOT; fake claude + cargo on PATH."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-test-"))
        # Git repo (PROJECT_ROOT)
        _git("init", "-q", "-b", "master", cwd=self.tmp)
        _git("config", "user.email", "t@example.com", cwd=self.tmp)
        _git("config", "user.name", "Test Test", cwd=self.tmp)
        (self.tmp / "README.md").write_text("baseline\n")
        _git("add", "-A", cwd=self.tmp)
        _git("commit", "-q", "-m", "init", cwd=self.tmp)

        # Pipeline layout inside the repo
        tickets = self.tmp / "new_pipeline" / "tickets"
        staging = self.tmp / "new_pipeline" / "staging"
        prompts = self.tmp / "new_pipeline" / "prompts"
        tickets.mkdir(parents=True)
        prompts.mkdir(parents=True)
        # Copy the real test-writer prompt into the temp prompts dir.
        real_prompt = (
            _THIS.parents[1] / "prompts" / "test-writer.md"
        ).read_text()
        (prompts / "test-writer.md").write_text(real_prompt)

        # Fake binaries on PATH
        bindir = self.tmp / "bin"
        bindir.mkdir()
        (bindir / "claude").write_text(_FAKE_CLAUDE)
        (bindir / "claude").chmod(0o755)
        (bindir / "cargo").write_text(_FAKE_CARGO)
        (bindir / "cargo").chmod(0o755)

        self.tickets = tickets
        self.staging = staging
        self.prompts = prompts

        new_path = f"{bindir}:{os.environ.get('PATH', '')}"
        self._patches = [
            patch.object(utils, "PROJECT_ROOT", self.tmp),
            patch.object(utils, "TICKETS_DIR", tickets),
            patch.object(utils, "ARCHIVE_DIR", tickets / "archive"),
            patch.object(utils, "STAGING_DIR", staging),
            patch.object(utils, "PROMPTS_DIR", prompts),
            patch.object(utils, "LOGS_DIR", self.tmp / "logs"),
            patch.object(utils, "WORKTREES_DIR", self.tmp / ".worktrees"),
            patch.object(
                oracle,
                "get_oracle_text",
                lambda card: f"[oracle for {card}]",
            ),
            patch.dict(os.environ, {"PATH": new_path}, clear=False),
        ]
        for p in self._patches:
            p.start()
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        for k in (
            "FAKE_AGENT_PAYLOAD_FILE",
            "FAKE_AGENT_TEST_SRC_FILE",
            "FAKE_AGENT_FAIL",
            "FAKE_CARGO_EXIT",
            "FAKE_CARGO_STDOUT",
            "FAKE_CARGO_STDERR",
        ):
            os.environ.pop(k, None)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _script_agent(
        self, payload: dict, test_src: str = _GOOD_TEST_SRC,
    ) -> None:
        """Point the fake claude at a canned TestReport + Rust source."""
        p = self.tmp / "payload.json"
        p.write_text(json.dumps(payload))
        os.environ["FAKE_AGENT_PAYLOAD_FILE"] = str(p)
        s = self.tmp / "test_src.rs"
        s.write_text(test_src)
        os.environ["FAKE_AGENT_TEST_SRC_FILE"] = str(s)

    # ── happy path ─────────────────────────────────────────────────

    def test_success_marks_ticket_tested(self) -> None:
        """Agent confirms + cargo fails-with-assertion → ticket is TESTED."""
        _write_ticket_file(
            self.tickets, "foo-01", slug="test_my_scenario",
        )
        test_file_rel = "mtg-engine/tests/pipeline_bugs_foo_01.rs"
        self._script_agent({
            "test_file": test_file_rel,
            "tests": [
                {
                    "slug": "test_my_scenario",
                    "status": "confirmed",
                    "test_name": "test_my_scenario",
                    "assertion_message": "expected 21, got 20",
                },
            ],
        })
        # Cargo: test fails with assertion → bug confirmed.
        os.environ["FAKE_CARGO_EXIT"] = "101"
        os.environ["FAKE_CARGO_STDOUT"] = (
            "---- test_my_scenario stdout ----\n"
            "assertion `left == right` failed\n"
            " left: 20\nright: 21\n"
        )

        test_mod.cmd_test(_args("foo-01"))

        t = Ticket.load("foo-01")
        self.assertIs(t.status, Status.TESTED)
        fm = t.frontmatter
        self.assertTrue(fm.test_run_id.endswith("-foo-01-test"))
        self.assertEqual(fm.test_model, "opus")
        self.assertEqual(fm.test_tokens, 150)
        self.assertEqual(fm.test_file, test_file_rel)
        self.assertEqual(len(fm.tested_sha), 40)
        self.assertIsNotNone(fm.tested_at)
        # Results section appended.
        self.assertIn("## Test Run Results", t.body)
        self.assertIn("test_my_scenario", t.body)

    # ── skip + no-bug + error paths ────────────────────────────────

    def test_non_new_ticket_is_skipped(self) -> None:
        """A ticket whose status isn't `new` is skipped, not modified."""
        _write_ticket_file(self.tickets, "closed-01", status="closed")
        # Closed tickets live in archive/ in the real flow; here we've
        # written directly into active for simplicity — the guard is on
        # status, not filesystem location.
        test_mod.cmd_test(_args("closed-01"))
        t = Ticket.load("closed-01")
        self.assertIs(t.status, Status.CLOSED)
        self.assertEqual(t.frontmatter.test_run_id, "")

    def test_all_rejected_closes_not_a_bug(self) -> None:
        """Agent rejects every scenario → closed with reason not_a_bug."""
        _write_ticket_file(self.tickets, "fp-01")
        self._script_agent({
            "test_file": "mtg-engine/tests/pipeline_bugs_fp_01.rs",
            "tests": [
                {
                    "slug": "test_my_scenario",
                    "status": "rejected",
                    "explanation": "passes against current code",
                },
            ],
        })
        # No cargo calls expected — all rejected → no validation needed.

        test_mod.cmd_test(_args("fp-01"))

        t = Ticket.load("fp-01")
        self.assertIs(t.status, Status.CLOSED)
        self.assertEqual(t.frontmatter.closed_reason, "not_a_bug")
        self.assertTrue(t.status.is_terminal)
        self.assertTrue((self.tickets / "archive" / "fp-01.md").exists())
        self.assertFalse((self.tickets / "fp-01.md").exists())
        self.assertIn("## Test Run Results", t.body)
        self.assertIn("passes against current code", t.body)

    def test_agent_error_keeps_ticket_new(self) -> None:
        """A persistent agent error → ticket stays `new`, nothing stamped."""
        _write_ticket_file(self.tickets, "err-01")
        os.environ["FAKE_AGENT_FAIL"] = "agent blew up"

        test_mod.cmd_test(_args("err-01"))

        t = Ticket.load("err-01")
        self.assertIs(t.status, Status.NEW)
        self.assertEqual(t.frontmatter.test_run_id, "")

    # ── strict aggregation ────────────────────────────────────────

    def test_mixed_confirmed_and_rejected_marks_mixed(
        self,
    ) -> None:
        """Any non-confirmed scenario blocks the whole ticket → mixed."""
        (self.tickets / "mixed-01.md").write_text(textwrap.dedent(
            """\
            ---
            id: mixed-01
            status: new
            card: Fake Card
            ---

            ## Tests

            ### scenario_a
            Scenario: one.

            ### scenario_b
            Scenario: two.
            """
        ))
        self._script_agent({
            "test_file": "mtg-engine/tests/pipeline_bugs_mixed_01.rs",
            "tests": [
                {
                    "slug": "scenario_a", "status": "confirmed",
                    "test_name": "test_scenario_a",
                    "assertion_message": "x",
                },
                {
                    "slug": "scenario_b", "status": "rejected",
                    "explanation": "not actually a bug",
                },
            ],
        })
        # Make cargo validate scenario_a (the confirmed one) as a real bug.
        os.environ["FAKE_CARGO_EXIT"] = "101"
        os.environ["FAKE_CARGO_STDOUT"] = (
            "assertion `left == right` failed\n  left: 20\n right: 21\n"
        )

        test_mod.cmd_test(_args("mixed-01"))

        t = Ticket.load("mixed-01")
        self.assertIs(t.status, Status.MIXED)
        self.assertTrue(t.status.is_terminal)

    def test_all_needs_engine_work_marks_engine_blocked(self) -> None:
        """Every scenario needs engine surface → terminal `engine_blocked`."""
        _write_ticket_file(self.tickets, "blocked-01")
        self._script_agent({
            "test_file": "mtg-engine/tests/pipeline_bugs_blocked_01.rs",
            "tests": [
                {
                    "slug": "test_my_scenario",
                    "status": "needs_engine_work",
                    "explanation": (
                        "would need combat::apply_damage_with_source()"
                    ),
                },
            ],
        })

        test_mod.cmd_test(_args("blocked-01"))

        t = Ticket.load("blocked-01")
        self.assertIs(t.status, Status.ENGINE_BLOCKED)
        self.assertTrue(t.status.is_terminal)
        self.assertIn("needs_engine_work", t.body)
        self.assertIn("combat::apply_damage_with_source", t.body)

    def test_mixed_needs_engine_and_rejected_marks_mixed(self) -> None:
        """Mix of needs_engine_work and rejected (no confirmed) → mixed, not engine_blocked."""
        (self.tickets / "mix-nr-01.md").write_text(textwrap.dedent(
            """\
            ---
            id: mix-nr-01
            status: new
            card: Foo
            ---

            ## Tests

            ### scenario_a

            ### scenario_b
            """
        ))
        self._script_agent({
            "test_file": "mtg-engine/tests/pipeline_bugs_mix_nr_01.rs",
            "tests": [
                {
                    "slug": "scenario_a", "status": "needs_engine_work",
                    "explanation": "need foo::bar()",
                },
                {
                    "slug": "scenario_b", "status": "rejected",
                    "explanation": "auditor misread the code",
                },
            ],
        })

        test_mod.cmd_test(_args("mix-nr-01"))

        t = Ticket.load("mix-nr-01")
        self.assertIs(t.status, Status.MIXED)

    # ── arg validation ────────────────────────────────────────────

    def test_empty_tickets_raises(self) -> None:
        """`--tickets ' , '` → ValueError."""
        with self.assertRaises(ValueError):
            test_mod.cmd_test(_args(" , "))


if __name__ == "__main__":
    unittest.main(verbosity=2)
