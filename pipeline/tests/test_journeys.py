"""Integration tests for the pipeline user journeys (J1–J10).

Each test sets up a real temp git repo + pipeline directory layout,
patches cli module constants to point at it, scripts the agent responses
(faking what the real Claude CLI would have written to staging + the
worktree), and asserts the resulting ticket state on disk.

No actual Claude invocations, no cargo, no mtg-engine source — just the
pipeline state machine exercised end to end on real files.

Run with:
    python3 -m unittest pipeline.tests.test_journeys -v
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from unittest.mock import patch

# Make `import pipeline.X` work regardless of the cwd the tests run in.
_THIS = Path(__file__).resolve()
_REPO_ROOT = _THIS.parents[2]
sys.path.insert(0, str(_REPO_ROOT))

from pipeline import agent, utils, validate, worktree  # noqa: E402
from pipeline.commands import close as close_mod  # noqa: E402
from pipeline.commands import consolidate as consolidate_mod  # noqa: E402
from pipeline.commands import fix as fix_mod  # noqa: E402
from pipeline.commands import merge as merge_mod  # noqa: E402
from pipeline.commands import retry as retry_mod  # noqa: E402
from pipeline.commands import test as test_mod  # noqa: E402
from pipeline.models import Ticket, parse_tests_section  # noqa: E402
from pipeline.state import CloseReason, Status  # noqa: E402
from pipeline.validate import FixValidation, TestValidation  # noqa: E402

# ──────────────────────────────────────────────────────────────────
# Fixture: a disposable pipeline project rooted in a temp directory.
# ──────────────────────────────────────────────────────────────────


class PipelineEnv:
    """Real files, real git, fake agents.

    Creates a tmp dir with pipeline/ subdirs, initializes a git repo,
    installs always-pass stub validation scripts, and patches
    `pipeline.cli` so all path-sensitive functions target this env.
    """

    def __init__(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="pipeline-journey-"))
        self.pipeline = self.tmp / "pipeline"
        (self.pipeline / "tickets").mkdir(parents=True)
        (self.pipeline / "staging").mkdir()
        (self.pipeline / "logs").mkdir()
        (self.pipeline / "metrics").mkdir()
        (self.pipeline / "prompts").mkdir()
        (self.pipeline / "scripts").mkdir()
        # Prompts are read verbatim and prepended to the per-agent prompt.
        # The fake agent ignores them, but the file reads must succeed.
        for name in (
            "auditor.md",
            "test-writer.md",
            "fixer.md",
            "dedup.md",
            "auditor-insights.md",
        ):
            (self.pipeline / "prompts" / name).write_text(f"# fake {name}\n")
        # Per-agent templates — the fake agent infers the ticket id from
        # the prompt, so the placeholders the real templates expand
        # (notably `Ticket ID: {tid}`) need to survive into the prompt.
        peragent_dir = self.pipeline / "prompts"
        (peragent_dir / "audit.peragent.md").write_text(
            "## Card to audit: {card}\nrun_id: {run_id}\n"
        )
        (peragent_dir / "test.peragent.md").write_text(
            "{ticket_body}\n### Ticket ID: {tid}\n"
        )
        (peragent_dir / "fix.peragent.md").write_text(
            "{ticket_body}\n### Ticket ID: {tid}\n"
        )
        (peragent_dir / "dedup.peragent.md").write_text("{tickets_section}\n")

        # Stub validate.validate_test / validate_fix so we don't invoke cargo.
        # The journey tests exercise state-machine wiring, not Rust builds.
        # Also stub the legacy shell scripts (still referenced by one
        # validator path if Python code decides to shell out).
        for name in ("validate_test.sh", "validate_fix.sh"):
            p = self.pipeline / "scripts" / name
            p.write_text("#!/bin/bash\nexit 0\n")
            p.chmod(p.stat().st_mode | stat.S_IEXEC)

        # Real git repo with one baseline commit.
        self._git("init", "-q", "-b", "master")
        self._git("config", "user.email", "t@example.com")
        self._git("config", "user.name", "Journey Test")
        (self.tmp / "README.md").write_text("baseline\n")
        self._git("add", "-A")
        self._git("commit", "-q", "-m", "init")

        # Patch pipeline.utils' path constants to this env. Every other
        # module reads via `utils.X`, so patching the module attrs is enough.
        self._patchers = []
        new_paths = {
            "PROJECT_ROOT": self.tmp,
            "PIPELINE_DIR": self.pipeline,
            "TICKETS_DIR": self.pipeline / "tickets",
            "ARCHIVE_DIR": self.pipeline / "tickets" / "archive",
            "STAGING_DIR": self.pipeline / "staging",
            "PROMPTS_DIR": self.pipeline / "prompts",
            "SCRIPTS_DIR": self.pipeline / "scripts",
            "METRICS_DIR": self.pipeline / "metrics",
            "LOGS_DIR": self.pipeline / "logs",
            "WORKTREES_DIR": self.tmp / ".worktrees",
            "AGENT_SETTINGS": self.pipeline / "agent-settings.json",
        }
        for name, val in new_paths.items():
            p = patch.object(utils, name, val)
            p.start()
            self._patchers.append(p)

        # Oracle lookup shells out to scripts/oracle_lookup.py. Stub.
        p = patch.object(
            utils, "get_oracle_text", lambda name: f"[fake oracle for {name}]"
        )
        p.start()
        self._patchers.append(p)

        # Scripted agent responses — set per-journey via install_agent().
        self._agent_script: list[Callable[[Path, str], None]] = []
        p = patch.object(agent, "run_agent_in", self._fake_run_agent_in)
        p.start()
        self._patchers.append(p)
        p = patch.object(agent, "run_agent", self._fake_run_agent)
        p.start()
        self._patchers.append(p)

        # Short-circuit validate.* so we don't depend on cargo being on PATH.
        # Tests that want a rejection-at-validation path can re-patch.
        p = patch.object(
            validate,
            "validate_test",
            lambda *a, **k: TestValidation(True, "stubbed"),
        )
        p.start()
        self._patchers.append(p)
        p = patch.object(
            validate,
            "validate_fix",
            lambda *a, **k: FixValidation(True, "stubbed"),
        )
        p.start()
        self._patchers.append(p)

        # Merge's post-merge cargo test is also subprocess-based; stub it
        # so the journey tests don't invoke cargo.
        p = patch.object(merge_mod, "_tests_pass_at_head", lambda _t: True)
        p.start()
        self._patchers.append(p)

    # Public helpers ────────────────────────────────────────────────
    def install_agent(self, script: list[Callable[[Path, str], None]]) -> None:
        """Install an ordered list of agent-response callables.

        Each callable runs for one agent invocation in order. It receives
        the worktree path and the ticket id (best-effort, parsed from
        the prompt) and is responsible for writing the staging file and
        committing any code changes.
        """
        self._agent_script = list(script)

    def cleanup(self) -> None:
        """Stop every patch and delete the temp directory."""
        for p in reversed(self._patchers):
            p.stop()
        # Make read-only .git files deletable on macOS.
        shutil.rmtree(self.tmp, ignore_errors=True)

    # Convenience accessors ─────────────────────────────────────────
    def read_ticket(self, ticket_id: str):
        """Load and return the Ticket for `ticket_id` from this env."""
        return Ticket.load(ticket_id)

    def status(self, ticket_id: str):
        """Return the on-disk status of `ticket_id` as a Status enum."""
        return self.read_ticket(ticket_id).status

    # Internals ─────────────────────────────────────────────────────
    def _git(
        self, *args: str, cwd: Path | None = None
    ) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["git", *args],
            cwd=str(cwd or self.tmp),
            capture_output=True,
            text=True,
            check=True,
        )

    def _fake_run_agent_in(self, prompt: str, cwd: Path, *a, **kw) -> dict:
        tid = _extract_ticket_id(prompt)
        if not self._agent_script:
            raise AssertionError(
                "agent invoked but no scripted responses remain "
                f"(prompt head: {prompt[:120]!r})"
            )
        fn = self._agent_script.pop(0)
        fn(Path(cwd), tid)
        return {
            "returncode": 0,
            "tokens": 42,
            "tool_uses": 1,
            "duration": 1,
            "is_error": False,
            "error_message": None,
        }

    def _fake_run_agent(self, prompt: str, *a, **kw) -> dict:
        return self._fake_run_agent_in(prompt, self.tmp, *a, **kw)


def _extract_ticket_id(prompt: str) -> str:
    """Pull the ticket id out of a per-agent prompt. Tolerant of the two.

    shapes actually used: `### Ticket ID: X` (test-writer) and the
    per-agent-prompt header the audit command uses. Returns '' if
    absent.
    """
    m = re.search(r"Ticket ID:\s*(\S+)", prompt)
    if m:
        return m.group(1).strip()
    m = re.search(r"pipeline_bugs_(\w+)", prompt)
    if m:
        return m.group(1).replace("_", "-")
    return ""


# ──────────────────────────────────────────────────────────────────
# Agent-response building blocks.
#
# A response writes a staging file (what the real agent produces) and
# commits any worktree changes (what the real agent does via `git add
# && git commit` in the shared prompt). The cli's validation logic
# then consumes the staging file and inspects the worktree.
# ──────────────────────────────────────────────────────────────────


def _commit_all(wt: Path, msg: str) -> None:
    subprocess.run(
        ["git", "add", "-A"], cwd=str(wt), check=True, capture_output=True
    )
    subprocess.run(
        ["git", "commit", "-q", "-m", msg],
        cwd=str(wt),
        check=True,
        capture_output=True,
    )


def _write_test_staging(
    wt: Path, tid: str, test_file_rel: str, per_test: list[dict]
) -> None:
    staging = wt / "pipeline" / "staging" / f"{tid}-test.json"
    staging.parent.mkdir(parents=True, exist_ok=True)
    staging.write_text(
        json.dumps({"test_file": test_file_rel, "tests": per_test}, indent=2)
    )


def tester_confirms(test_slugs: list[str] | None = None):
    """Agent script: write a Rust test file, commit it, emit a staging.

    file that confirms every slug in the ticket's ## Tests section.
    """

    def _fn(wt: Path, tid: str) -> None:
        tid_snake = tid.replace("-", "_")
        test_file_rel = f"mtg-engine/tests/pipeline_bugs_{tid_snake}.rs"
        test_file = wt / test_file_rel
        test_file.parent.mkdir(parents=True, exist_ok=True)
        test_file.write_text("#[test]\nfn placeholder() { assert!(true); }\n")
        _commit_all(wt, f"Add tests for {tid}")
        slugs = test_slugs or _slugs_from_ticket(tid)
        _write_test_staging(
            wt,
            tid,
            test_file_rel,
            [
                {
                    "slug": s,
                    "status": "confirmed",
                    "test_name": f"test_{s}",
                    "assertion_message": "fake",
                    "explanation": "scripted",
                }
                for s in slugs
            ],
        )

    return _fn


def tester_blocks_on_engine(reason: str = "needs new API in foo.rs:123"):
    """Agent script: report the test can't be written without an engine.

    change. Commits nothing
    writes staging with per-test blocked.
    """

    def _fn(wt: Path, tid: str) -> None:
        slugs = _slugs_from_ticket(tid)
        test_file_rel = (
            f"mtg-engine/tests/pipeline_bugs_{tid.replace('-', '_')}.rs"
        )
        _write_test_staging(
            wt,
            tid,
            test_file_rel,
            [
                {
                    "slug": s,
                    "status": "blocked",
                    "test_name": "",
                    "blocked_by": reason,
                    "explanation": "scripted",
                }
                for s in slugs
            ],
        )

    return _fn


def tester_all_false_positive():
    """Agent script: every test entry comes back Status=rejected.

    Aggregates to false_positive.
    """

    def _fn(wt: Path, tid: str) -> None:
        slugs = _slugs_from_ticket(tid)
        test_file_rel = (
            f"mtg-engine/tests/pipeline_bugs_{tid.replace('-', '_')}.rs"
        )
        _write_test_staging(
            wt,
            tid,
            test_file_rel,
            [
                {
                    "slug": s,
                    "status": "rejected",
                    "test_name": "",
                    "explanation": "test passes — bug is not real",
                }
                for s in slugs
            ],
        )

    return _fn


def fixer_succeeds():
    """Agent script: write a source change, commit, emit fixed staging."""

    def _fn(wt: Path, tid: str) -> None:
        f = wt / "mtg-engine" / "src" / "engine.rs"
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(f"// fix for {tid}\n")
        _commit_all(wt, f"Fix {tid}")
        staging = wt / "pipeline" / "staging" / f"{tid}-fix.json"
        staging.parent.mkdir(parents=True, exist_ok=True)
        staging.write_text(
            json.dumps(
                {
                    "status": "fixed",
                    "files_changed": ["mtg-engine/src/engine.rs"],
                    "description": "Fixed the dispatcher filter.",
                },
                indent=2,
            )
        )

    return _fn


def fixer_fails(reason: str = "could not satisfy all tests"):
    """Agent script: commit nothing new, emit failed staging."""

    def _fn(wt: Path, tid: str) -> None:
        staging = wt / "pipeline" / "staging" / f"{tid}-fix.json"
        staging.parent.mkdir(parents=True, exist_ok=True)
        staging.write_text(
            json.dumps(
                {
                    "status": "failed",
                    "files_changed": [],
                    "description": reason,
                },
                indent=2,
            )
        )

    return _fn


def _slugs_from_ticket(tid: str) -> list[str]:
    t = Ticket.load(tid)
    return [entry.slug for entry in parse_tests_section(t.body)]


# ──────────────────────────────────────────────────────────────────
# Ticket factory.
# ──────────────────────────────────────────────────────────────────


def _make_ticket(
    env: PipelineEnv,
    tid: str,
    *,
    card: str = "Fake Card",
    slugs: list[str] | None = None,
) -> None:
    """Write a minimal `status: new` ticket with a ## Tests section."""
    slugs = slugs or ["baseline"]
    body_lines = [
        f"# {card} — fake bug",
        "",
        "## Description",
        f"A scripted bug for {card}.",
        "",
        "## Engine path",
        "- engine.rs:1",
        "",
        "## Tests",
        "",
    ]
    for s in slugs:
        body_lines.append(f"### {s}")
        body_lines.append("Source ticket: (new)")
        body_lines.append("Implementation: (not yet written)")
        body_lines.append(f"Scenario: exercise {s}.")
        body_lines.append("")
    Ticket.create(
        tid, status=Status.NEW, card=card, body="\n".join(body_lines)
    )


# ──────────────────────────────────────────────────────────────────
# Driver helpers — run cli commands with Namespace args, like main does.
# ──────────────────────────────────────────────────────────────────


def _run_test(env: PipelineEnv, tid: str, *, parallelism: int = 1) -> None:
    test_mod.cmd_test(
        argparse.Namespace(
            tickets=tid,
            parallelism=parallelism,
            model="fake",
            effort="fake",
            dry_run=False,
        )
    )


def _run_fix(env: PipelineEnv, tid: str) -> None:
    fix_mod.cmd_fix(
        argparse.Namespace(
            ticket=tid, model="fake", effort="fake", dry_run=False
        )
    )


def _run_retry(env: PipelineEnv, tid: str, *, to=None, force=False) -> None:
    retry_mod.cmd_retry(argparse.Namespace(ticket_id=tid, to=to, force=force))


def _run_close(env: PipelineEnv, tid: str, *, note: str | None = None) -> None:
    close_mod.cmd_close(argparse.Namespace(ticket_id=tid, note=note))


def _run_merge(env: PipelineEnv, tid: str) -> None:
    merge_mod.cmd_merge(
        argparse.Namespace(
            ticket_id=tid, model="fake", effort="fake", dry_run=False
        )
    )


# ══════════════════════════════════════════════════════════════════
# Test cases — one per journey.
# ══════════════════════════════════════════════════════════════════


class JourneyTest(unittest.TestCase):
    """End-to-end journey tests driving cmd_* against a scripted agent."""

    def setUp(self) -> None:
        """Stand up a fresh PipelineEnv per test; auto-cleanup on teardown."""
        self.env = PipelineEnv()
        self.addCleanup(self.env.cleanup)

    def test_j1_happy_path(self) -> None:
        """Happy path — new → tested → fixed → shipped."""
        tid = "olivia-01"
        _make_ticket(self.env, tid, card="Olivia Voldaren")
        self.env.install_agent([tester_confirms(), fixer_succeeds()])

        _run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.TESTED)
        self.assertTrue(self.env.read_ticket(tid).frontmatter.tested_sha)

        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIXED)
        self.assertTrue(self.env.read_ticket(tid).frontmatter.fixed_sha)

        _run_merge(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.SHIPPED)
        fm = self.env.read_ticket(tid).frontmatter
        self.assertTrue(fm.shipped_sha)
        self.assertTrue(fm.shipped_at)

    # J2: Fix fails; retry keeps the tests, redoes the fix.
    def test_j2_fix_fails_then_retry_to_tested(self) -> None:
        """Fix fails; retry keeps the tests, redoes the fix."""
        tid = "foo-01"
        _make_ticket(self.env, tid)
        self.env.install_agent(
            [
                tester_confirms(),
                fixer_fails("first attempt broken"),
                fixer_succeeds(),
            ]
        )

        _run_test(self.env, tid)
        tested_sha = self.env.read_ticket(tid).frontmatter.tested_sha

        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIX_FAILED)

        _run_retry(self.env, tid)  # default for fix_failed → --to tested
        self.assertEqual(self.env.status(tid), Status.TESTED)
        fm = self.env.read_ticket(tid).frontmatter
        self.assertEqual(fm.tested_sha, tested_sha)
        self.assertFalse(fm.fixed_at)
        self.assertFalse(fm.fix_run_id)
        # Prior fix result archived
        body = self.env.read_ticket(tid).body
        self.assertIn("## Attempt 1", body)
        # Branch HEAD points at tested_sha
        branch = worktree.branch_for(tid)
        self.assertEqual(worktree.branch_head(branch), tested_sha)

        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIXED)

    # J3: Tests were wrong — retry --to new, redo everything.
    def test_j3_retry_to_new_throws_out_tests(self) -> None:
        """Tests were wrong — retry --to new, redo everything."""
        tid = "bar-01"
        _make_ticket(self.env, tid)
        self.env.install_agent(
            [
                tester_confirms(),
                fixer_fails("tests assert wrong thing"),
                tester_confirms(),
                fixer_succeeds(),
            ]
        )

        _run_test(self.env, tid)
        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIX_FAILED)

        _run_retry(self.env, tid, to=Status.NEW)
        self.assertEqual(self.env.status(tid), Status.NEW)
        fm = self.env.read_ticket(tid).frontmatter
        # All phase-specific fields cleared
        for k in (
            "tested_sha",
            "fixed_sha",
            "test_run_id",
            "fix_run_id",
            "test_file",
            "worktree",
        ):
            self.assertFalse(getattr(fm, k), f"{k} should be cleared")
        # Worktree gone
        self.assertFalse(worktree.dir_for(tid).exists())
        # Implementation lines reset
        body = self.env.read_ticket(tid).body
        self.assertIn("Implementation: (not yet written)", body)
        # Prior runs archived
        self.assertIn("## Attempt 1", body)

        _run_test(self.env, tid)
        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIXED)

    # J4: Test-writer blocked on engine — escape hatch → status stays new
    #     with allow_engine_edits=true.
    def test_j4_engine_edit_escape_hatch(self) -> None:
        """Test-writer blocked on engine — escape hatch, status stays new with allow_engine_edits=true."""
        tid = "baz-01"
        _make_ticket(self.env, tid)
        self.env.install_agent(
            [
                tester_blocks_on_engine("need new pub fn in engine.rs:55"),
                tester_confirms(),  # retry: agent has engine access this time
                fixer_succeeds(),
            ]
        )

        _run_test(self.env, tid)
        fm = self.env.read_ticket(tid).frontmatter
        self.assertEqual(fm.status, Status.NEW)
        self.assertTrue(fm.allow_engine_edits)
        self.assertTrue(fm.engine_block_at)
        body = self.env.read_ticket(tid).body
        self.assertIn("## Engine Change Needed", body)
        self.assertIn("need new pub fn", body)

        # Re-running `test` picks it up; no retry needed because status
        # is already `new`.
        _run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.TESTED)
        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIXED)

    # J5: Bug not real — all tests rejected → false_positive (terminal).
    def test_j5_false_positive(self) -> None:
        """Bug not real — all tests rejected → false_positive (terminal)."""
        tid = "qux-01"
        _make_ticket(self.env, tid)
        self.env.install_agent([tester_all_false_positive()])

        _run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FALSE_POSITIVE)

        # retry without --force refuses
        with self.assertRaises(SystemExit) as ctx:
            _run_retry(self.env, tid)
        self.assertEqual(ctx.exception.code, 1)
        self.assertEqual(self.env.status(tid), Status.FALSE_POSITIVE)

        # retry --force --to new re-opens it
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        _run_retry(self.env, tid, to=Status.NEW, force=True)
        self.assertEqual(self.env.status(tid), Status.NEW)
        _run_test(self.env, tid)
        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIXED)

    # J6: Dedup absorbs a ticket — children → closed/absorbed.
    def test_j6_dedup_absorbs(self) -> None:
        """Dedup absorbs a ticket — children → closed/absorbed."""
        tid = "widget-01"
        _make_ticket(self.env, tid, card="Widget", slugs=["widget_scenario"])

        # Hand-craft a consolidation staging file (what the dedup agent
        # would produce) and run cmd_consolidate directly.
        staging = self.env.pipeline / "staging" / "consolidation-w-target.json"
        staging.write_text(
            json.dumps(
                {
                    "slug": "w-target",
                    "title": "Merged target check",
                    "description": "One engine cause.",
                    "engine_path": ["engine.rs:42"],
                    "tests": [
                        {
                            "slug": "widget_scenario",
                            "source_ticket": tid,
                            "scenario": "exercise the engine.",
                        }
                    ],
                },
                indent=2,
            )
        )

        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(staging), keep_input=False, dry_run=False
            )
        )

        # Child absorbed
        fm = self.env.read_ticket(tid).frontmatter
        self.assertEqual(fm.status, Status.CLOSED)
        self.assertEqual(fm.closed_reason, CloseReason.ABSORBED.value)
        self.assertEqual(fm.absorbed_into, "merged-w-target-01")
        self.assertTrue(fm.closed_at)
        # Parent minted
        parent = self.env.read_ticket("merged-w-target-01")
        self.assertEqual(parent.status, Status.NEW)
        self.assertEqual(parent.card, "multiple")

    # J7: A merged-* parent's fix fails; retry it like any other ticket.
    def test_j7_retry_on_merged_parent(self) -> None:
        """A merged-* parent whose fix fails retries like any other ticket."""
        # Build: two children → consolidate → parent → test/fix flow.
        for child in ("c1-01", "c2-01"):
            _make_ticket(self.env, child, card=child, slugs=[f"{child}_slug"])
        staging = self.env.pipeline / "staging" / "consolidation-p.json"
        staging.write_text(
            json.dumps(
                {
                    "slug": "p",
                    "title": "Parent",
                    "description": "d",
                    "engine_path": ["a.rs:1"],
                    "tests": [
                        {
                            "slug": "c1_01_slug",
                            "source_ticket": "c1-01",
                            "scenario": "s.",
                        },
                        {
                            "slug": "c2_01_slug",
                            "source_ticket": "c2-01",
                            "scenario": "s.",
                        },
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(staging), keep_input=False, dry_run=False
            )
        )
        parent_id = "merged-p-01"

        self.env.install_agent(
            [
                tester_confirms(),
                fixer_fails(),
                fixer_succeeds(),
            ]
        )
        _run_test(self.env, parent_id)
        self.assertEqual(self.env.status(parent_id), Status.TESTED)

        _run_fix(self.env, parent_id)
        self.assertEqual(self.env.status(parent_id), Status.FIX_FAILED)

        _run_retry(self.env, parent_id)  # → tested
        self.assertEqual(self.env.status(parent_id), Status.TESTED)

        _run_fix(self.env, parent_id)
        self.assertEqual(self.env.status(parent_id), Status.FIXED)

    # J8: Merge conflict — merge fails, ticket stays `fixed`.
    def test_j8_merge_conflict(self) -> None:
        """Merge conflict — merge fails, ticket stays fixed."""
        tid = "conflict-01"
        _make_ticket(self.env, tid)
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        _run_test(self.env, tid)
        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIXED)

        # Create a conflicting commit on master.
        (self.env.tmp / "mtg-engine" / "src").mkdir(parents=True, exist_ok=True)
        (self.env.tmp / "mtg-engine" / "src" / "engine.rs").write_text(
            "// conflicting change on master\n"
        )
        subprocess.run(["git", "add", "-A"], cwd=self.env.tmp, check=True)
        subprocess.run(
            ["git", "commit", "-q", "-m", "master conflict"],
            cwd=self.env.tmp,
            check=True,
        )

        # The cli merge path tries to merge the branch, fails cleanly,
        # and leaves the ticket `fixed` for the human to resolve.
        _run_merge(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIXED)
        # Clean up the in-progress merge state left by git so tearDown works.
        subprocess.run(["git", "merge", "--abort"], cwd=self.env.tmp)

    # J9: `close` — manual abandonment.
    def test_j9_close_abandoned(self) -> None:
        """— manual abandonment."""
        tid = "giveup-01"
        _make_ticket(self.env, tid)
        self.env.install_agent(
            [
                tester_confirms(),
                fixer_fails("I surrender"),
            ]
        )
        _run_test(self.env, tid)
        _run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.FIX_FAILED)
        self.assertTrue(worktree.dir_for(tid).exists())

        _run_close(self.env, tid, note="not worth pursuing")

        fm = self.env.read_ticket(tid).frontmatter
        self.assertEqual(fm.status, Status.CLOSED)
        self.assertEqual(fm.closed_reason, CloseReason.ABANDONED.value)
        self.assertEqual(fm.closed_note, "not worth pursuing")
        self.assertFalse(worktree.dir_for(tid).exists())

    # Bonus: malformed JSON staging → retry → success.
    def test_staging_schema_retry(self) -> None:
        """Malformed JSON staging → retry → success."""
        tid = "schema-01"
        _make_ticket(self.env, tid)

        def bad_json(wt: Path, tid: str) -> None:
            staging = wt / "pipeline" / "staging" / f"{tid}-test.json"
            staging.parent.mkdir(parents=True, exist_ok=True)
            # Missing required `tests` field
            staging.write_text('{"test_file": "foo.rs"}')

        self.env.install_agent(
            [
                bad_json,
                tester_confirms(),
            ]
        )
        _run_test(self.env, tid)
        # First attempt rejected for schema; second attempt succeeds.
        self.assertEqual(self.env.status(tid), Status.TESTED)

    # ── Archive + consolidate enforcement ──────────────────────────

    def test_terminal_transitions_archive(self) -> None:
        """Close / merge-to-shipped / consolidate-absorb each archive the ticket file."""
        """Close, merge-to-shipped, and consolidate-absorb should each

        physically move the ticket file into pipeline/tickets/archive/.
        """
        active = self.env.pipeline / "tickets"
        archive = active / "archive"

        # close → closed/abandoned
        _make_ticket(self.env, "abd-01")
        self.assertTrue((active / "abd-01.md").exists())
        _run_close(self.env, "abd-01")
        self.assertFalse((active / "abd-01.md").exists())
        self.assertTrue((archive / "abd-01.md").exists())

        # merge → shipped
        _make_ticket(self.env, "shp-01")
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        _run_test(self.env, "shp-01")
        _run_fix(self.env, "shp-01")
        self.assertTrue((active / "shp-01.md").exists())
        _run_merge(self.env, "shp-01")
        self.assertFalse((active / "shp-01.md").exists())
        self.assertTrue((archive / "shp-01.md").exists())

        # consolidate → absorbed sources archived, parent stays active
        _make_ticket(self.env, "abs-01", slugs=["abs_slug"])
        staging = self.env.pipeline / "staging" / "consolidation-abs.json"
        staging.write_text(
            json.dumps(
                {
                    "slug": "abs",
                    "title": "Absorb",
                    "description": "root cause",
                    "engine_path": ["x.rs:1"],
                    "tests": [
                        {
                            "slug": "abs_slug",
                            "source_ticket": "abs-01",
                            "scenario": "exercise",
                        }
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(staging), keep_input=False, dry_run=False
            )
        )
        self.assertFalse((active / "abs-01.md").exists())
        self.assertTrue((archive / "abs-01.md").exists())
        self.assertTrue(
            (active / "merged-abs-01.md").exists()
        )  # parent is open

    def test_consolidate_closed_source_ticket_becomes_metadata(self) -> None:
        """A per-test source_ticket pointing at an already-closed ticket is kept as metadata, not re-absorbed."""
        """When a per-test `source_ticket` points at an already-closed

        ticket, Python treats it as coverage metadata (not an absorption
        target). The closed ticket's status is unchanged. This supports
        the common case of copying tests verbatim from a merged-* parent
        that carries closed-card-ticket source_ticket pointers.
        """
        # Two cards: one already closed (the metadata case), one open
        # (the legitimate absorption target).
        _make_ticket(self.env, "closed-01", slugs=["closed_slug"])
        _run_close(self.env, "closed-01")
        _make_ticket(self.env, "open-01", slugs=["open_slug"])

        staging = self.env.pipeline / "staging" / "consolidation-meta.json"
        staging.write_text(
            json.dumps(
                {
                    "slug": "meta",
                    "title": "Metadata kept",
                    "description": "d",
                    "engine_path": ["x.rs:1"],
                    "tests": [
                        {
                            "slug": "closed_slug",
                            "source_ticket": "closed-01",
                            "scenario": "copied from elsewhere",
                        },
                        {
                            "slug": "open_slug",
                            "source_ticket": "open-01",
                            "scenario": "fresh absorption",
                        },
                    ],
                },
                indent=2,
            )
        )

        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(staging), keep_input=False, dry_run=False
            )
        )

        # Parent created
        self.assertEqual(self.env.status("merged-meta-01"), Status.NEW)
        # Open ticket absorbed
        self.assertEqual(self.env.status("open-01"), Status.CLOSED)
        # Closed ticket unchanged — still closed, still points where it
        # pointed before
        self.assertEqual(self.env.status("closed-01"), Status.CLOSED)
        closed_fm = self.env.read_ticket("closed-01").frontmatter
        self.assertNotEqual(closed_fm.absorbed_into, "merged-meta-01")

    def test_consolidate_rejects_closed_also_closes(self) -> None:
        """Closed tickets cannot appear in Also closes (explicit absorption requests must be open)."""
        """`also_closes` entries are EXPLICIT absorption requests —

        they must be currently-open tickets. Listing an already-closed
        id there is a protocol violation that gets rejected hard.
        """
        _make_ticket(self.env, "live-01", slugs=["live_slug"])
        _make_ticket(self.env, "dead-01", slugs=["dead_slug"])
        _run_close(self.env, "dead-01")

        staging = self.env.pipeline / "staging" / "consolidation-bad.json"
        staging.write_text(
            json.dumps(
                {
                    "slug": "bad",
                    "title": "Bad",
                    "description": "d",
                    "engine_path": ["x.rs:1"],
                    "tests": [
                        {
                            "slug": "live_slug",
                            "source_ticket": "live-01",
                            "scenario": "exercise",
                        },
                    ],
                    "also_closes": ["dead-01"],
                },
                indent=2,
            )
        )

        with self.assertRaises(consolidate_mod.ConsolidateError):
            consolidate_mod.cmd_consolidate(
                argparse.Namespace(
                    input=str(staging), keep_input=False, dry_run=False
                )
            )

        # No parent created; neither ticket's state changed.
        self.assertFalse(
            (self.env.pipeline / "tickets" / "merged-bad-01.md").exists()
        )
        self.assertEqual(self.env.status("live-01"), Status.NEW)
        self.assertEqual(self.env.status("dead-01"), Status.CLOSED)

    def test_auto_numbering_sees_archive(self) -> None:
        """New ticket id allocation skips ids that live in archive."""
        """Auto-numbering of new ticket ids must skip ids that live in

        archive, so a re-audit of an already-audited card doesn't stomp
        on an archived ticket's id.
        """
        # Seed: create -01 and archive it.
        _make_ticket(self.env, "numbering-01")
        _run_close(self.env, "numbering-01")
        archive = self.env.pipeline / "tickets" / "archive"
        self.assertTrue((archive / "numbering-01.md").exists())

        # Next merged consolidation using the same stem base should take
        # -02, not reuse -01. Exercise via consolidate (easier than a
        # full cmd_audit) — the same all_ticket_paths helper drives both.
        _make_ticket(self.env, "src-01", slugs=["s"])
        staging = self.env.pipeline / "staging" / "consolidation-numbering.json"
        staging.write_text(
            json.dumps(
                {
                    "slug": "numbering",
                    "title": "Numbering",
                    "description": "d",
                    "engine_path": ["x.rs:1"],
                    "tests": [
                        {
                            "slug": "s",
                            "source_ticket": "src-01",
                            "scenario": "e",
                        }
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(staging), keep_input=False, dry_run=False
            )
        )
        # merged-numbering-01 was a prior ticket (archived) so new id is -02
        self.assertTrue(
            (
                self.env.pipeline / "tickets" / "archive" / "numbering-01.md"
            ).exists()
        )
        self.assertTrue(
            (self.env.pipeline / "tickets" / "merged-numbering-01.md").exists()
        )
        # Now create a second consolidation using the same slug: must get -02
        _make_ticket(self.env, "src-02", slugs=["s2"])
        staging2 = (
            self.env.pipeline / "staging" / "consolidation-numbering2.json"
        )
        staging2.write_text(
            json.dumps(
                {
                    "slug": "numbering",
                    "title": "Numbering 2",
                    "description": "d",
                    "engine_path": ["x.rs:1"],
                    "tests": [
                        {
                            "slug": "s2",
                            "source_ticket": "src-02",
                            "scenario": "e",
                        }
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(staging2), keep_input=False, dry_run=False
            )
        )
        self.assertTrue(
            (self.env.pipeline / "tickets" / "merged-numbering-02.md").exists()
        )
        # Ship the first parent so it moves to archive, then do a third
        # consolidation — must take -03, not reuse -01 or -02.
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        _run_test(self.env, "merged-numbering-01")
        _run_fix(self.env, "merged-numbering-01")
        _run_merge(self.env, "merged-numbering-01")
        self.assertEqual(self.env.status("merged-numbering-01"), Status.SHIPPED)
        self.assertTrue(
            (
                self.env.pipeline
                / "tickets"
                / "archive"
                / "merged-numbering-01.md"
            ).exists()
        )

        _make_ticket(self.env, "src-03", slugs=["s3"])
        staging3 = (
            self.env.pipeline / "staging" / "consolidation-numbering3.json"
        )
        staging3.write_text(
            json.dumps(
                {
                    "slug": "numbering",
                    "title": "Numbering 3",
                    "description": "d",
                    "engine_path": ["x.rs:1"],
                    "tests": [
                        {
                            "slug": "s3",
                            "source_ticket": "src-03",
                            "scenario": "e",
                        }
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(staging3), keep_input=False, dry_run=False
            )
        )
        self.assertTrue(
            (self.env.pipeline / "tickets" / "merged-numbering-03.md").exists()
        )

    def test_list_tickets_spans_active_and_archive(self) -> None:
        """list_all reads both directories so reports see the full history."""
        """list_tickets reads both directories so reports/backlog

        queries see the full history.
        """
        _make_ticket(self.env, "live-01")
        _make_ticket(self.env, "dead-01")
        _run_close(self.env, "dead-01")

        all_tickets = Ticket.list_all()
        ids = {t.id for t in all_tickets}
        self.assertIn("live-01", ids)
        self.assertIn("dead-01", ids)

        # Filtering by status also sees the archived ones
        closed = Ticket.list_all(status=Status.CLOSED)
        self.assertEqual({t.id for t in closed}, {"dead-01"})
        new = Ticket.list_all(status=Status.NEW)
        self.assertEqual({t.id for t in new}, {"live-01"})

    def test_absorb_tested_source_inherits_worktree(self) -> None:
        """Absorbing a tested merged-* source into a deeper parent inherits its worktree + test file."""
        """Absorbing a single `tested` merged-* source into a deeper new

        parent: the worktree and branch are renamed, the test file is
        renamed on disk and committed, and inherited Implementation
        pointers appear in the new parent's body (pointing at the new
        test-file name). If all tests inherit, the new parent goes
        straight to status=tested.
        """
        # Set up: create a merged parent via consolidate, then run test
        # on it so its status becomes `tested`.
        for tid, slug in (("a-01", "a_slug"), ("b-01", "b_slug")):
            _make_ticket(self.env, tid, slugs=[slug])
        stg = self.env.pipeline / "staging" / "consolidation-first.json"
        stg.write_text(
            json.dumps(
                {
                    "slug": "first",
                    "title": "First",
                    "description": "d",
                    "engine_path": ["e.rs:1"],
                    "tests": [
                        {
                            "slug": "a_slug",
                            "source_ticket": "a-01",
                            "scenario": "s",
                        },
                        {
                            "slug": "b_slug",
                            "source_ticket": "b-01",
                            "scenario": "s",
                        },
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(input=str(stg), keep_input=False, dry_run=False)
        )
        parent1 = "merged-first-01"
        self.assertEqual(self.env.status(parent1), Status.NEW)

        # Test-writer runs on parent1 → status: tested
        self.env.install_agent([tester_confirms()])
        _run_test(self.env, parent1)
        self.assertEqual(self.env.status(parent1), Status.TESTED)
        parent1_test_file = self.env.read_ticket(parent1).test_file
        self.assertTrue(
            parent1_test_file.endswith("pipeline_bugs_merged_first_01.rs")
        )

        # Now dedup: absorb parent1 (via Also closes) + a new open card
        # ticket into a deeper parent2. The new parent's tests should
        # include copies of parent1's slugs verbatim PLUS a new slug
        # for the card ticket.
        _make_ticket(self.env, "c-01", slugs=["c_slug"])
        stg2 = self.env.pipeline / "staging" / "consolidation-deep.json"
        stg2.write_text(
            json.dumps(
                {
                    "slug": "deep",
                    "title": "Deep",
                    "description": "d",
                    "engine_path": ["e.rs:1"],
                    "tests": [
                        # Copies verbatim from parent1 (slugs preserved)
                        {
                            "slug": "a_slug",
                            "source_ticket": "a-01",
                            "scenario": "s",
                        },
                        {
                            "slug": "b_slug",
                            "source_ticket": "b-01",
                            "scenario": "s",
                        },
                        # New entry for the card ticket
                        {
                            "slug": "c_slug",
                            "source_ticket": "c-01",
                            "scenario": "s",
                        },
                    ],
                    "also_closes": [parent1],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(input=str(stg2), keep_input=False, dry_run=False)
        )

        parent2 = "merged-deep-01"
        # parent1 archived
        self.assertEqual(self.env.status(parent1), Status.CLOSED)
        # parent2 has mixed implementations — a_slug & b_slug inherited,
        # c_slug still needs implementing. So status stays `new`.
        self.assertEqual(self.env.status(parent2), Status.NEW)
        fm2 = self.env.read_ticket(parent2).frontmatter
        self.assertEqual(fm2.inherited_from, parent1)
        new_test_file = fm2.test_file
        self.assertTrue(
            new_test_file.endswith("pipeline_bugs_merged_deep_01.rs")
        )

        # Check body has inherited Implementation pointers on a/b and
        # (not yet written) on c, all pointing at the NEW test file.
        body2 = self.env.read_ticket(parent2).body
        self.assertIn(f"Implementation: {new_test_file}::test_a_slug", body2)
        self.assertIn(f"Implementation: {new_test_file}::test_b_slug", body2)
        self.assertIn("Implementation: (not yet written)", body2)

        # The worktree was renamed: parent1's dir is gone, parent2's exists.
        self.assertFalse(worktree.dir_for(parent1).exists())
        self.assertTrue(worktree.dir_for(parent2).exists())
        # The test file was renamed on disk.
        wt = worktree.dir_for(parent2)
        self.assertTrue((wt / new_test_file).exists())
        self.assertFalse((wt / parent1_test_file).exists())

    def test_absorb_multiple_tested_merges(self) -> None:
        """Multiple tested sources merge into a single tested parent.

        Each source's test file is concatenated into the new parent's
        test file, every tested source's worktree is torn down, and the
        parent lands at `tested` with inherited implementations for all
        proposed slugs.
        """
        # Two separately-tested merged parents.
        for slug in ("x", "y"):
            _make_ticket(self.env, f"{slug}-01", slugs=[f"{slug}_slug"])
        for slug in ("x", "y"):
            stg = self.env.pipeline / "staging" / f"cons-{slug}.json"
            stg.write_text(
                json.dumps(
                    {
                        "slug": slug,
                        "title": slug,
                        "description": "d",
                        "engine_path": ["e.rs:1"],
                        "tests": [
                            {
                                "slug": f"{slug}_slug",
                                "source_ticket": f"{slug}-01",
                                "scenario": "s",
                            }
                        ],
                    },
                    indent=2,
                )
            )
            consolidate_mod.cmd_consolidate(
                argparse.Namespace(
                    input=str(stg), keep_input=False, dry_run=False
                )
            )
        self.env.install_agent([tester_confirms(), tester_confirms()])
        _run_test(self.env, "merged-x-01")
        _run_test(self.env, "merged-y-01")
        self.assertEqual(self.env.status("merged-x-01"), Status.TESTED)
        self.assertEqual(self.env.status("merged-y-01"), Status.TESTED)

        # Absorb both tested parents into one deeper parent.
        stg3 = self.env.pipeline / "staging" / "cons-both.json"
        stg3.write_text(
            json.dumps(
                {
                    "slug": "both",
                    "title": "Both",
                    "description": "d",
                    "engine_path": ["e.rs:1"],
                    "tests": [
                        {
                            "slug": "x_slug",
                            "source_ticket": "x-01",
                            "scenario": "s",
                        },
                        {
                            "slug": "y_slug",
                            "source_ticket": "y-01",
                            "scenario": "s",
                        },
                    ],
                    "also_closes": ["merged-x-01", "merged-y-01"],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(stg3), keep_input=False, dry_run=False
            )
        )

        parent = "merged-both-01"
        # Both sources absorbed.
        self.assertEqual(self.env.status("merged-x-01"), Status.CLOSED)
        self.assertEqual(self.env.status("merged-y-01"), Status.CLOSED)
        # Parent minted at tested with inherited impls.
        self.assertEqual(self.env.status(parent), Status.TESTED)
        fm = self.env.read_ticket(parent).frontmatter
        self.assertIn("merged-x-01", fm.inherited_from)
        self.assertIn("merged-y-01", fm.inherited_from)
        new_test_file = fm.test_file
        self.assertTrue(
            new_test_file.endswith("pipeline_bugs_merged_both_01.rs")
        )
        body = self.env.read_ticket(parent).body
        self.assertIn(f"Implementation: {new_test_file}::test_x_slug", body)
        self.assertIn(f"Implementation: {new_test_file}::test_y_slug", body)
        # Source worktrees torn down; parent has its own.
        self.assertFalse(worktree.dir_for("merged-x-01").exists())
        self.assertFalse(worktree.dir_for("merged-y-01").exists())
        self.assertTrue(worktree.dir_for(parent).exists())

    def test_absorb_single_fixed_drops_fix_commits(self) -> None:
        """A single fixed source is absorbed; fix commits are dropped.

        The parent inherits the source's worktree + test file but rewinds
        the branch to `tested_sha`, so the fix commit never travels
        forward. The parent lands at `tested`, not `fixed`.
        """
        _make_ticket(self.env, "fx-01", slugs=["fx_slug"])
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        _run_test(self.env, "fx-01")
        tested_sha = self.env.read_ticket("fx-01").frontmatter.tested_sha
        _run_fix(self.env, "fx-01")
        self.assertEqual(self.env.status("fx-01"), Status.FIXED)

        stg = self.env.pipeline / "staging" / "cons-fx.json"
        stg.write_text(
            json.dumps(
                {
                    "slug": "fx",
                    "title": "Absorb fixed",
                    "description": "d",
                    "engine_path": ["e.rs:1"],
                    "tests": [
                        {
                            "slug": "fx_slug",
                            "source_ticket": "fx-01",
                            "scenario": "s",
                        }
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(stg), keep_input=False, dry_run=False
            )
        )

        parent = "merged-fx-01"
        self.assertEqual(self.env.status("fx-01"), Status.CLOSED)
        self.assertEqual(self.env.status(parent), Status.TESTED)
        # Parent's branch HEAD was rewound past the fix commit before
        # the rename-commit landed, so the fix diff is gone.
        wt = worktree.dir_for(parent)
        self.assertTrue(wt.exists())
        self.assertFalse((wt / "mtg-engine" / "src" / "engine.rs").exists())
        # Branch parent commit chain passes through tested_sha.
        log = subprocess.run(
            ["git", "log", "--format=%H"],
            cwd=str(wt),
            capture_output=True,
            text=True,
            check=True,
        ).stdout.split()
        self.assertIn(tested_sha, log)

    def test_absorb_fixed_plus_new_source(self) -> None:
        """One fixed + one new source → parent lands at new (single-source rename).

        The new-status ticket contributes no worktree, so this still
        goes through the single-source rename fast path. The fixed
        source's fix commits are dropped (branch reset to tested_sha).
        """
        _make_ticket(self.env, "fx-01", slugs=["fx_slug"])
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        _run_test(self.env, "fx-01")
        _run_fix(self.env, "fx-01")
        self.assertEqual(self.env.status("fx-01"), Status.FIXED)

        _make_ticket(self.env, "new-01", slugs=["new_slug"])
        self.assertEqual(self.env.status("new-01"), Status.NEW)

        stg = self.env.pipeline / "staging" / "cons-fn.json"
        stg.write_text(
            json.dumps(
                {
                    "slug": "fn",
                    "title": "Fixed + new",
                    "description": "d",
                    "engine_path": ["e.rs:1"],
                    "tests": [
                        {
                            "slug": "fx_slug",
                            "source_ticket": "fx-01",
                            "scenario": "s",
                        },
                        {
                            "slug": "new_slug",
                            "source_ticket": "new-01",
                            "scenario": "s",
                        },
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(stg), keep_input=False, dry_run=False
            )
        )

        parent = "merged-fn-01"
        self.assertEqual(self.env.status("fx-01"), Status.CLOSED)
        self.assertEqual(self.env.status("new-01"), Status.CLOSED)
        # Parent lands at `new` because new-01 contributes an unimplemented
        # test. The fx_slug impl is inherited; new_slug is "(not yet written)".
        self.assertEqual(self.env.status(parent), Status.NEW)
        fm = self.env.read_ticket(parent).frontmatter
        self.assertEqual(fm.inherited_from, "fx-01")
        body = self.env.read_ticket(parent).body
        self.assertIn(f"Implementation: {fm.test_file}::test_fx_slug", body)
        self.assertIn("Implementation: (not yet written)", body)
        # Fix artifact dropped.
        wt = worktree.dir_for(parent)
        self.assertFalse((wt / "mtg-engine" / "src" / "engine.rs").exists())

    def test_absorb_fixed_plus_tested_merges(self) -> None:
        """One fixed + one tested source → parent lands at tested, fix dropped."""
        _make_ticket(self.env, "f-01", slugs=["f_slug"])
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        _run_test(self.env, "f-01")
        _run_fix(self.env, "f-01")
        self.assertEqual(self.env.status("f-01"), Status.FIXED)

        _make_ticket(self.env, "t-01", slugs=["t_slug"])
        self.env.install_agent([tester_confirms()])
        _run_test(self.env, "t-01")
        self.assertEqual(self.env.status("t-01"), Status.TESTED)

        stg = self.env.pipeline / "staging" / "cons-ft.json"
        stg.write_text(
            json.dumps(
                {
                    "slug": "ft",
                    "title": "Fixed + tested",
                    "description": "d",
                    "engine_path": ["e.rs:1"],
                    "tests": [
                        {
                            "slug": "f_slug",
                            "source_ticket": "f-01",
                            "scenario": "s",
                        },
                        {
                            "slug": "t_slug",
                            "source_ticket": "t-01",
                            "scenario": "s",
                        },
                    ],
                },
                indent=2,
            )
        )
        consolidate_mod.cmd_consolidate(
            argparse.Namespace(
                input=str(stg), keep_input=False, dry_run=False
            )
        )

        parent = "merged-ft-01"
        self.assertEqual(self.env.status("f-01"), Status.CLOSED)
        self.assertEqual(self.env.status("t-01"), Status.CLOSED)
        self.assertEqual(self.env.status(parent), Status.TESTED)
        # The merged test file exists and references both impls.
        fm = self.env.read_ticket(parent).frontmatter
        new_test_file = fm.test_file
        body = self.env.read_ticket(parent).body
        self.assertIn(f"Implementation: {new_test_file}::test_f_slug", body)
        self.assertIn(f"Implementation: {new_test_file}::test_t_slug", body)
        # Fix artifact from f-01 was dropped during merge (engine.rs never
        # entered the parent's branch).
        wt = worktree.dir_for(parent)
        self.assertFalse((wt / "mtg-engine" / "src" / "engine.rs").exists())

    def test_absorb_rejects_two_fixed(self) -> None:
        """Two fixed sources → rejected (combining fix commit-sets is OOS)."""
        for slug in ("p", "q"):
            _make_ticket(self.env, f"{slug}-01", slugs=[f"{slug}_slug"])
            self.env.install_agent([tester_confirms(), fixer_succeeds()])
            _run_test(self.env, f"{slug}-01")
            _run_fix(self.env, f"{slug}-01")
            self.assertEqual(self.env.status(f"{slug}-01"), Status.FIXED)

        stg = self.env.pipeline / "staging" / "cons-pq.json"
        stg.write_text(
            json.dumps(
                {
                    "slug": "pq",
                    "title": "Two fixed",
                    "description": "d",
                    "engine_path": ["e.rs:1"],
                    "tests": [
                        {
                            "slug": "p_slug",
                            "source_ticket": "p-01",
                            "scenario": "s",
                        },
                        {
                            "slug": "q_slug",
                            "source_ticket": "q-01",
                            "scenario": "s",
                        },
                    ],
                },
                indent=2,
            )
        )
        with self.assertRaises(consolidate_mod.ConsolidateError):
            consolidate_mod.cmd_consolidate(
                argparse.Namespace(
                    input=str(stg), keep_input=False, dry_run=False
                )
            )
        self.assertEqual(self.env.status("p-01"), Status.FIXED)
        self.assertEqual(self.env.status("q-01"), Status.FIXED)
        self.assertFalse(
            (self.env.pipeline / "tickets" / "merged-pq-01.md").exists()
        )

    def test_absorb_rejects_fix_failed(self) -> None:
        """`fix_failed` sources are off-limits — post-mortem must survive.

        `new`/`tested`/`fixed` absorb cleanly; `fix_failed` carries the
        post-mortem commits that are the only artifact of a failed run,
        so silently losing them on absorption would hide real info.
        The user must `retry --to tested` first.
        """
        _make_ticket(self.env, "ff-01", slugs=["ff_slug"])
        self.env.install_agent([tester_confirms(), fixer_fails()])
        _run_test(self.env, "ff-01")
        _run_fix(self.env, "ff-01")
        self.assertEqual(self.env.status("ff-01"), Status.FIX_FAILED)

        stg = self.env.pipeline / "staging" / "cons-ff.json"
        stg.write_text(
            json.dumps(
                {
                    "slug": "ff",
                    "title": "Reject",
                    "description": "d",
                    "engine_path": ["e.rs:1"],
                    "tests": [
                        {
                            "slug": "ff_slug",
                            "source_ticket": "ff-01",
                            "scenario": "s",
                        }
                    ],
                },
                indent=2,
            )
        )
        with self.assertRaises(consolidate_mod.ConsolidateError):
            consolidate_mod.cmd_consolidate(
                argparse.Namespace(
                    input=str(stg), keep_input=False, dry_run=False
                )
            )
        self.assertEqual(self.env.status("ff-01"), Status.FIX_FAILED)

    # J10: Status guards — test refuses non-new, fix refuses non-tested.
    def test_j10_status_guards(self) -> None:
        """Status guards — test refuses non-new, fix refuses non-tested."""
        tid = "guard-01"
        _make_ticket(self.env, tid)
        self.env.install_agent([tester_confirms()])
        _run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.TESTED)

        # `test` on a tested ticket is a no-op: cmd_test filters out
        # any ticket whose status != new and prints a message.
        _run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), Status.TESTED)

        # `fix` on a new ticket is blocked with a clear error.
        tid2 = "guard-02"
        _make_ticket(self.env, tid2)
        with self.assertRaises(SystemExit):
            _run_fix(self.env, tid2)
        self.assertEqual(self.env.status(tid2), Status.NEW)


if __name__ == "__main__":
    unittest.main(verbosity=2)
