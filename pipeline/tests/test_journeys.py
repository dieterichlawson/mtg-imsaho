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
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Callable
from unittest.mock import patch

# Make `import pipeline.cli` work regardless of the cwd the tests run in.
_THIS = Path(__file__).resolve()
_REPO_ROOT = _THIS.parents[2]
sys.path.insert(0, str(_REPO_ROOT))

from pipeline import cli  # noqa: E402


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
        for name in ("auditor.md", "test-writer.md", "fixer.md",
                     "dedup.md", "auditor-insights.md"):
            (self.pipeline / "prompts" / name).write_text(f"# fake {name}\n")
        # Per-agent templates — the fake agent infers the ticket id from
        # the prompt, so the placeholders the real templates expand
        # (notably `Ticket ID: {tid}`) need to survive into the prompt.
        peragent_dir = self.pipeline / "prompts"
        (peragent_dir / "audit.peragent.md").write_text(
            "## Card to audit: {card}\nrun_id: {run_id}\n")
        (peragent_dir / "test.peragent.md").write_text(
            "{ticket_body}\n### Ticket ID: {tid}\n")
        (peragent_dir / "fix.peragent.md").write_text(
            "{ticket_body}\n### Ticket ID: {tid}\n")
        (peragent_dir / "dedup.peragent.md").write_text(
            "{tickets_section}\n")

        # Stub validation scripts — always succeed.
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

        # Patch the cli module's path-dependent globals to this env.
        self._patchers = []
        paths = {
            "PROJECT_ROOT": self.tmp,
            "PIPELINE_DIR": self.pipeline,
            "TICKETS_DIR":  self.pipeline / "tickets",
            "ARCHIVE_DIR":  self.pipeline / "tickets" / "archive",
            "STAGING_DIR":  self.pipeline / "staging",
            "PROMPTS_DIR":  self.pipeline / "prompts",
            "SCRIPTS_DIR":  self.pipeline / "scripts",
            "METRICS_DIR":  self.pipeline / "metrics",
            "LOGS_DIR":     self.pipeline / "logs",
            "WORKTREES_DIR": self.tmp / ".worktrees",
        }
        for name, val in paths.items():
            p = patch.object(cli, name, val)
            p.start()
            self._patchers.append(p)

        # Oracle text is looked up by `get_oracle_text` which shells out
        # to scripts/oracle_lookup.py. Stub it out.
        p = patch.object(cli, "get_oracle_text",
                         lambda name: f"[fake oracle for {name}]")
        p.start()
        self._patchers.append(p)

        # Scripted agent responses — set per-journey via install_agent().
        self._agent_script: list[Callable[[Path, str], None]] = []
        p = patch.object(cli, "run_agent_in", self._fake_run_agent_in)
        p.start()
        self._patchers.append(p)
        # cmd_audit uses run_agent (no cwd); the default fake would also
        # serve. We don't exercise cmd_audit in these journeys.
        p = patch.object(cli, "run_agent", self._fake_run_agent)
        p.start()
        self._patchers.append(p)

    # Public helpers ────────────────────────────────────────────────
    def install_agent(self, script: list[Callable[[Path, str], None]]) -> None:
        """Install an ordered list of agent-response callables.

        Each callable runs for one agent invocation in order. It receives
        the worktree path and the ticket id (best-effort, parsed from
        the prompt) and is responsible for writing the staging file and
        committing any code changes."""
        self._agent_script = list(script)

    def cleanup(self) -> None:
        for p in reversed(self._patchers):
            p.stop()
        # Make read-only .git files deletable on macOS.
        shutil.rmtree(self.tmp, ignore_errors=True)

    # Convenience accessors ─────────────────────────────────────────
    def ticket_path(self, ticket_id: str) -> Path:
        # Mirror cli.ticket_path: look active first, then archive.
        return cli.ticket_path(ticket_id)

    def read_ticket(self, ticket_id: str) -> dict:
        return cli.parse_ticket(self.ticket_path(ticket_id))

    def status(self, ticket_id: str) -> str:
        return self.read_ticket(ticket_id)["frontmatter"].get("status", "")

    def write_ticket(self, ticket_id: str, fm: dict, body: str) -> None:
        cli.write_ticket(ticket_id, fm, body)

    # Internals ─────────────────────────────────────────────────────
    def _git(self, *args: str, cwd: Path | None = None) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["git", *args], cwd=str(cwd or self.tmp),
            capture_output=True, text=True, check=True,
        )

    def _fake_run_agent_in(self, prompt: str, cwd: Path, *a, **kw) -> dict:
        tid = _extract_ticket_id(prompt)
        if not self._agent_script:
            raise AssertionError(
                "agent invoked but no scripted responses remain "
                f"(prompt head: {prompt[:120]!r})")
        fn = self._agent_script.pop(0)
        fn(Path(cwd), tid)
        return {"returncode": 0, "tokens": 42, "tool_uses": 1,
                "duration": 1, "is_error": False, "error_message": None}

    def _fake_run_agent(self, prompt: str, *a, **kw) -> dict:
        return self._fake_run_agent_in(prompt, self.tmp, *a, **kw)


def _extract_ticket_id(prompt: str) -> str:
    """Pull the ticket id out of a per-agent prompt. Tolerant of the two
    shapes actually used: `### Ticket ID: X` (test-writer) and the
    per-agent-prompt header the audit command uses. Returns '' if
    absent."""
    import re
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
    subprocess.run(["git", "add", "-A"], cwd=str(wt), check=True,
                   capture_output=True)
    subprocess.run(["git", "commit", "-q", "-m", msg], cwd=str(wt),
                   check=True, capture_output=True)


def _write_test_staging(wt: Path, tid: str, test_file_rel: str,
                        per_test: list[dict]) -> None:
    import json as _json
    staging = wt / "pipeline" / "staging" / f"{tid}-test.json"
    staging.parent.mkdir(parents=True, exist_ok=True)
    staging.write_text(_json.dumps(
        {"test_file": test_file_rel, "tests": per_test}, indent=2))


def tester_confirms(test_slugs: list[str] | None = None):
    """Agent script: write a Rust test file, commit it, emit a staging
    file that confirms every slug in the ticket's ## Tests section."""
    def _fn(wt: Path, tid: str) -> None:
        tid_snake = tid.replace("-", "_")
        test_file_rel = f"mtg-engine/tests/pipeline_bugs_{tid_snake}.rs"
        test_file = wt / test_file_rel
        test_file.parent.mkdir(parents=True, exist_ok=True)
        test_file.write_text(
            "#[test]\nfn placeholder() { assert!(true); }\n")
        _commit_all(wt, f"Add tests for {tid}")
        slugs = test_slugs or _slugs_from_ticket(tid)
        _write_test_staging(wt, tid, test_file_rel, [
            {"slug": s, "status": "confirmed", "test_name": f"test_{s}",
             "assertion_message": "fake", "explanation": "scripted"}
            for s in slugs])
    return _fn


def tester_blocks_on_engine(reason: str = "needs new API in foo.rs:123"):
    """Agent script: report the test can't be written without an engine
    change. Commits nothing; writes staging with per-test blocked."""
    def _fn(wt: Path, tid: str) -> None:
        slugs = _slugs_from_ticket(tid)
        test_file_rel = f"mtg-engine/tests/pipeline_bugs_{tid.replace('-','_')}.rs"
        _write_test_staging(wt, tid, test_file_rel, [
            {"slug": s, "status": "blocked", "test_name": "",
             "blocked_by": reason, "explanation": "scripted"}
            for s in slugs])
    return _fn


def tester_all_false_positive():
    """Agent script: every test entry comes back Status=rejected.
    Aggregates to false_positive."""
    def _fn(wt: Path, tid: str) -> None:
        slugs = _slugs_from_ticket(tid)
        test_file_rel = f"mtg-engine/tests/pipeline_bugs_{tid.replace('-','_')}.rs"
        _write_test_staging(wt, tid, test_file_rel, [
            {"slug": s, "status": "rejected", "test_name": "",
             "explanation": "test passes — bug is not real"}
            for s in slugs])
    return _fn


def fixer_succeeds():
    """Agent script: write a source change, commit, emit fixed staging."""
    def _fn(wt: Path, tid: str) -> None:
        import json as _json
        f = wt / "mtg-engine" / "src" / "engine.rs"
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(f"// fix for {tid}\n")
        _commit_all(wt, f"Fix {tid}")
        staging = wt / "pipeline" / "staging" / f"{tid}-fix.json"
        staging.parent.mkdir(parents=True, exist_ok=True)
        staging.write_text(_json.dumps({
            "status": "fixed",
            "files_changed": ["mtg-engine/src/engine.rs"],
            "description": "Fixed the dispatcher filter.",
        }, indent=2))
    return _fn


def fixer_fails(reason: str = "could not satisfy all tests"):
    """Agent script: commit nothing new, emit failed staging."""
    def _fn(wt: Path, tid: str) -> None:
        import json as _json
        staging = wt / "pipeline" / "staging" / f"{tid}-fix.json"
        staging.parent.mkdir(parents=True, exist_ok=True)
        staging.write_text(_json.dumps({
            "status": "failed",
            "files_changed": [],
            "description": reason,
        }, indent=2))
    return _fn


def _slugs_from_ticket(tid: str) -> list[str]:
    body = cli.parse_ticket(cli.ticket_path(tid))["body"]
    return [t["slug"] for t in cli._parse_tests_section(body)]


# ──────────────────────────────────────────────────────────────────
# Ticket factory.
# ──────────────────────────────────────────────────────────────────

def make_ticket(env: PipelineEnv, tid: str, *,
                card: str = "Fake Card",
                slugs: list[str] | None = None) -> None:
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
    fm = {
        "id": tid, "status": cli.STATUS_NEW, "card": card,
        "created": cli.now_iso(),
    }
    env.write_ticket(tid, fm, "\n".join(body_lines))


# ──────────────────────────────────────────────────────────────────
# Driver helpers — run cli commands with Namespace args, like main does.
# ──────────────────────────────────────────────────────────────────

def run_test(env: PipelineEnv, tid: str, *, parallelism: int = 1) -> None:
    cli.cmd_test(argparse.Namespace(
        tickets=tid, parallelism=parallelism,
        model="fake", effort="fake", dry_run=False))


def run_fix(env: PipelineEnv, tid: str) -> None:
    cli.cmd_fix(argparse.Namespace(
        ticket=tid, model="fake", effort="fake", dry_run=False))


def run_retry(env: PipelineEnv, tid: str, *, to=None, force=False) -> None:
    cli.cmd_retry(argparse.Namespace(
        ticket_id=tid, to=to, force=force))


def run_close(env: PipelineEnv, tid: str, *, note: str | None = None) -> None:
    cli.cmd_close(argparse.Namespace(ticket_id=tid, note=note))


def run_merge(env: PipelineEnv, tid: str) -> None:
    cli.cmd_merge(argparse.Namespace(
        ticket_id=tid, model="fake", effort="fake", dry_run=False))


# ══════════════════════════════════════════════════════════════════
# Test cases — one per journey.
# ══════════════════════════════════════════════════════════════════

class JourneyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.env = PipelineEnv()
        self.addCleanup(self.env.cleanup)

    # J1: Happy path — new → tested → fixed → shipped
    def test_j1_happy_path(self) -> None:
        tid = "olivia-01"
        make_ticket(self.env, tid, card="Olivia Voldaren")
        self.env.install_agent([tester_confirms(), fixer_succeeds()])

        run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_TESTED)
        self.assertTrue(self.env.read_ticket(tid)["frontmatter"].get("tested_sha"))

        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIXED)
        self.assertTrue(self.env.read_ticket(tid)["frontmatter"].get("fixed_sha"))

        run_merge(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_SHIPPED)
        fm = self.env.read_ticket(tid)["frontmatter"]
        self.assertTrue(fm.get("shipped_sha"))
        self.assertTrue(fm.get("shipped_at"))

    # J2: Fix fails; retry keeps the tests, redoes the fix.
    def test_j2_fix_fails_then_retry_to_tested(self) -> None:
        tid = "foo-01"
        make_ticket(self.env, tid)
        self.env.install_agent([
            tester_confirms(),
            fixer_fails("first attempt broken"),
            fixer_succeeds(),
        ])

        run_test(self.env, tid)
        tested_sha = self.env.read_ticket(tid)["frontmatter"]["tested_sha"]

        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIX_FAILED)

        run_retry(self.env, tid)  # default for fix_failed → --to tested
        self.assertEqual(self.env.status(tid), cli.STATUS_TESTED)
        fm = self.env.read_ticket(tid)["frontmatter"]
        self.assertEqual(fm.get("tested_sha"), tested_sha)
        self.assertNotIn("fixed_at", fm)
        self.assertNotIn("fix_run_id", fm)
        # Prior fix result archived
        body = self.env.read_ticket(tid)["body"]
        self.assertIn("## Attempt 1", body)
        # Branch HEAD points at tested_sha
        branch = cli.get_worktree_branch(tid)
        self.assertEqual(cli._branch_head_sha(branch), tested_sha)

        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIXED)

    # J3: Tests were wrong — retry --to new, redo everything.
    def test_j3_retry_to_new_throws_out_tests(self) -> None:
        tid = "bar-01"
        make_ticket(self.env, tid)
        self.env.install_agent([
            tester_confirms(),
            fixer_fails("tests assert wrong thing"),
            tester_confirms(),
            fixer_succeeds(),
        ])

        run_test(self.env, tid)
        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIX_FAILED)

        run_retry(self.env, tid, to=cli.STATUS_NEW)
        self.assertEqual(self.env.status(tid), cli.STATUS_NEW)
        fm = self.env.read_ticket(tid)["frontmatter"]
        # All phase-specific fields cleared
        for k in ("tested_sha", "fixed_sha", "test_run_id", "fix_run_id",
                  "test_file", "worktree"):
            self.assertNotIn(k, fm)
        # Worktree gone
        self.assertFalse(cli.get_worktree_dir(tid).exists())
        # Implementation lines reset
        body = self.env.read_ticket(tid)["body"]
        self.assertIn("Implementation: (not yet written)", body)
        # Prior runs archived
        self.assertIn("## Attempt 1", body)

        run_test(self.env, tid)
        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIXED)

    # J4: Test-writer blocked on engine — escape hatch → status stays new
    #     with allow_engine_edits=true.
    def test_j4_engine_edit_escape_hatch(self) -> None:
        tid = "baz-01"
        make_ticket(self.env, tid)
        self.env.install_agent([
            tester_blocks_on_engine("need new pub fn in engine.rs:55"),
            tester_confirms(),  # retry: agent has engine access this time
            fixer_succeeds(),
        ])

        run_test(self.env, tid)
        fm = self.env.read_ticket(tid)["frontmatter"]
        self.assertEqual(fm["status"], cli.STATUS_NEW)
        self.assertEqual(fm.get("allow_engine_edits"), "true")
        self.assertTrue(fm.get("engine_block_at"))
        body = self.env.read_ticket(tid)["body"]
        self.assertIn("## Engine Change Needed", body)
        self.assertIn("need new pub fn", body)

        # Re-running `test` picks it up; no retry needed because status
        # is already `new`.
        run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_TESTED)
        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIXED)

    # J5: Bug not real — all tests rejected → false_positive (terminal).
    def test_j5_false_positive(self) -> None:
        tid = "qux-01"
        make_ticket(self.env, tid)
        self.env.install_agent([tester_all_false_positive()])

        run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FALSE_POSITIVE)

        # retry without --force refuses
        with self.assertRaises(SystemExit) as ctx:
            run_retry(self.env, tid)
        self.assertEqual(ctx.exception.code, 1)
        self.assertEqual(self.env.status(tid), cli.STATUS_FALSE_POSITIVE)

        # retry --force --to new re-opens it
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        run_retry(self.env, tid, to=cli.STATUS_NEW, force=True)
        self.assertEqual(self.env.status(tid), cli.STATUS_NEW)
        run_test(self.env, tid)
        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIXED)

    # J6: Dedup absorbs a ticket — children → closed/absorbed.
    def test_j6_dedup_absorbs(self) -> None:
        tid = "widget-01"
        make_ticket(self.env, tid, card="Widget",
                    slugs=["widget_scenario"])

        # Hand-craft a consolidation staging file (what the dedup agent
        # would produce) and run cmd_consolidate directly.
        import json as _json
        staging = self.env.pipeline / "staging" / "consolidation-w-target.json"
        staging.write_text(_json.dumps({
            "slug": "w-target",
            "title": "Merged target check",
            "description": "One engine cause.",
            "engine_path": ["engine.rs:42"],
            "tests": [{
                "slug": "widget_scenario",
                "source_ticket": tid,
                "scenario": "exercise the engine.",
            }],
        }, indent=2))

        cli.cmd_consolidate(argparse.Namespace(
            input=str(staging), keep_input=False, dry_run=False))

        # Child absorbed
        fm = self.env.read_ticket(tid)["frontmatter"]
        self.assertEqual(fm["status"], cli.STATUS_CLOSED)
        self.assertEqual(fm["closed_reason"], cli.CLOSED_REASON_ABSORBED)
        self.assertEqual(fm["absorbed_into"], "merged-w-target-01")
        self.assertTrue(fm.get("closed_at"))
        # Parent minted
        parent = self.env.read_ticket("merged-w-target-01")
        self.assertEqual(parent["frontmatter"]["status"], cli.STATUS_NEW)
        self.assertEqual(parent["frontmatter"]["card"], "multiple")

    # J7: A merged-* parent's fix fails; retry it like any other ticket.
    def test_j7_retry_on_merged_parent(self) -> None:
        # Build: two children → consolidate → parent → test/fix flow.
        for child in ("c1-01", "c2-01"):
            make_ticket(self.env, child, card=child, slugs=[f"{child}_slug"])
        import json as _json
        staging = self.env.pipeline / "staging" / "consolidation-p.json"
        staging.write_text(_json.dumps({
            "slug": "p",
            "title": "Parent",
            "description": "d",
            "engine_path": ["a.rs:1"],
            "tests": [
                {"slug": "c1_01_slug", "source_ticket": "c1-01", "scenario": "s."},
                {"slug": "c2_01_slug", "source_ticket": "c2-01", "scenario": "s."},
            ],
        }, indent=2))
        cli.cmd_consolidate(argparse.Namespace(
            input=str(staging), keep_input=False, dry_run=False))
        parent_id = "merged-p-01"

        self.env.install_agent([
            tester_confirms(),
            fixer_fails(),
            fixer_succeeds(),
        ])
        run_test(self.env, parent_id)
        self.assertEqual(self.env.status(parent_id), cli.STATUS_TESTED)

        run_fix(self.env, parent_id)
        self.assertEqual(self.env.status(parent_id), cli.STATUS_FIX_FAILED)

        run_retry(self.env, parent_id)  # → tested
        self.assertEqual(self.env.status(parent_id), cli.STATUS_TESTED)

        run_fix(self.env, parent_id)
        self.assertEqual(self.env.status(parent_id), cli.STATUS_FIXED)

    # J8: Merge conflict — merge fails, ticket stays `fixed`.
    def test_j8_merge_conflict(self) -> None:
        tid = "conflict-01"
        make_ticket(self.env, tid)
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        run_test(self.env, tid)
        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIXED)

        # Create a conflicting commit on master.
        (self.env.tmp / "mtg-engine" / "src").mkdir(parents=True, exist_ok=True)
        (self.env.tmp / "mtg-engine" / "src" / "engine.rs").write_text(
            "// conflicting change on master\n")
        subprocess.run(["git", "add", "-A"], cwd=self.env.tmp, check=True)
        subprocess.run(["git", "commit", "-q", "-m", "master conflict"],
                       cwd=self.env.tmp, check=True)

        # The cli merge path tries to merge the branch, fails cleanly,
        # and leaves the ticket `fixed` for the human to resolve.
        run_merge(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIXED)
        # Clean up the in-progress merge state left by git so tearDown works.
        subprocess.run(["git", "merge", "--abort"], cwd=self.env.tmp)

    # J9: `close` — manual abandonment.
    def test_j9_close_abandoned(self) -> None:
        tid = "giveup-01"
        make_ticket(self.env, tid)
        self.env.install_agent([
            tester_confirms(),
            fixer_fails("I surrender"),
        ])
        run_test(self.env, tid)
        run_fix(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_FIX_FAILED)
        self.assertTrue(cli.get_worktree_dir(tid).exists())

        run_close(self.env, tid, note="not worth pursuing")

        fm = self.env.read_ticket(tid)["frontmatter"]
        self.assertEqual(fm["status"], cli.STATUS_CLOSED)
        self.assertEqual(fm["closed_reason"], cli.CLOSED_REASON_ABANDONED)
        self.assertEqual(fm.get("closed_note"), "not worth pursuing")
        self.assertFalse(cli.get_worktree_dir(tid).exists())

    # Bonus: malformed JSON staging → retry → success.
    def test_staging_schema_retry(self) -> None:
        tid = "schema-01"
        make_ticket(self.env, tid)

        def bad_json(wt: Path, tid: str) -> None:
            staging = wt / "pipeline" / "staging" / f"{tid}-test.json"
            staging.parent.mkdir(parents=True, exist_ok=True)
            # Missing required `tests` field
            staging.write_text('{"test_file": "foo.rs"}')

        self.env.install_agent([
            bad_json,
            tester_confirms(),
        ])
        run_test(self.env, tid)
        # First attempt rejected for schema; second attempt succeeds.
        self.assertEqual(self.env.status(tid), cli.STATUS_TESTED)

    # ── Archive + consolidate enforcement ──────────────────────────

    def test_terminal_transitions_archive(self) -> None:
        """Close, merge-to-shipped, and consolidate-absorb should each
        physically move the ticket file into pipeline/tickets/archive/."""
        active = self.env.pipeline / "tickets"
        archive = active / "archive"

        # close → closed/abandoned
        make_ticket(self.env, "abd-01")
        self.assertTrue((active / "abd-01.md").exists())
        run_close(self.env, "abd-01")
        self.assertFalse((active / "abd-01.md").exists())
        self.assertTrue((archive / "abd-01.md").exists())

        # merge → shipped
        make_ticket(self.env, "shp-01")
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        run_test(self.env, "shp-01")
        run_fix(self.env, "shp-01")
        self.assertTrue((active / "shp-01.md").exists())
        run_merge(self.env, "shp-01")
        self.assertFalse((active / "shp-01.md").exists())
        self.assertTrue((archive / "shp-01.md").exists())

        # consolidate → absorbed sources archived, parent stays active
        make_ticket(self.env, "abs-01", slugs=["abs_slug"])
        import json as _json
        staging = self.env.pipeline / "staging" / "consolidation-abs.json"
        staging.write_text(_json.dumps({
            "slug": "abs",
            "title": "Absorb",
            "description": "root cause",
            "engine_path": ["x.rs:1"],
            "tests": [{"slug": "abs_slug", "source_ticket": "abs-01",
                       "scenario": "exercise"}],
        }, indent=2))
        cli.cmd_consolidate(argparse.Namespace(
            input=str(staging), keep_input=False, dry_run=False))
        self.assertFalse((active / "abs-01.md").exists())
        self.assertTrue((archive / "abs-01.md").exists())
        self.assertTrue((active / "merged-abs-01.md").exists())  # parent is open

    def test_consolidate_closed_source_ticket_becomes_metadata(self) -> None:
        """When a per-test `source_ticket` points at an already-closed
        ticket, Python treats it as coverage metadata (not an absorption
        target). The closed ticket's status is unchanged. This supports
        the common case of copying tests verbatim from a merged-* parent
        that carries closed-card-ticket source_ticket pointers."""
        # Two cards: one already closed (the metadata case), one open
        # (the legitimate absorption target).
        make_ticket(self.env, "closed-01", slugs=["closed_slug"])
        run_close(self.env, "closed-01")
        make_ticket(self.env, "open-01", slugs=["open_slug"])

        import json as _json
        staging = self.env.pipeline / "staging" / "consolidation-meta.json"
        staging.write_text(_json.dumps({
            "slug": "meta",
            "title": "Metadata kept",
            "description": "d",
            "engine_path": ["x.rs:1"],
            "tests": [
                {"slug": "closed_slug", "source_ticket": "closed-01",
                 "scenario": "copied from elsewhere"},
                {"slug": "open_slug", "source_ticket": "open-01",
                 "scenario": "fresh absorption"},
            ],
        }, indent=2))

        cli.cmd_consolidate(argparse.Namespace(
            input=str(staging), keep_input=False, dry_run=False))

        # Parent created
        self.assertEqual(self.env.status("merged-meta-01"), cli.STATUS_NEW)
        # Open ticket absorbed
        self.assertEqual(self.env.status("open-01"), cli.STATUS_CLOSED)
        # Closed ticket unchanged — still closed, still points where it
        # pointed before
        self.assertEqual(self.env.status("closed-01"), cli.STATUS_CLOSED)
        closed_fm = self.env.read_ticket("closed-01")["frontmatter"]
        self.assertNotEqual(closed_fm.get("absorbed_into"), "merged-meta-01")

    def test_consolidate_rejects_closed_also_closes(self) -> None:
        """`also_closes` entries are EXPLICIT absorption requests —
        they must be currently-open tickets. Listing an already-closed
        id there is a protocol violation that gets rejected hard."""
        make_ticket(self.env, "live-01", slugs=["live_slug"])
        make_ticket(self.env, "dead-01", slugs=["dead_slug"])
        run_close(self.env, "dead-01")

        import json as _json
        staging = self.env.pipeline / "staging" / "consolidation-bad.json"
        staging.write_text(_json.dumps({
            "slug": "bad",
            "title": "Bad",
            "description": "d",
            "engine_path": ["x.rs:1"],
            "tests": [
                {"slug": "live_slug", "source_ticket": "live-01",
                 "scenario": "exercise"},
            ],
            "also_closes": ["dead-01"],
        }, indent=2))

        with self.assertRaises(SystemExit) as ctx:
            cli.cmd_consolidate(argparse.Namespace(
                input=str(staging), keep_input=False, dry_run=False))
        self.assertEqual(ctx.exception.code, 1)

        # No parent created; neither ticket's state changed.
        self.assertFalse((self.env.pipeline / "tickets" / "merged-bad-01.md").exists())
        self.assertEqual(self.env.status("live-01"), cli.STATUS_NEW)
        self.assertEqual(self.env.status("dead-01"), cli.STATUS_CLOSED)

    def test_auto_numbering_sees_archive(self) -> None:
        """Auto-numbering of new ticket ids must skip ids that live in
        archive, so a re-audit of an already-audited card doesn't stomp
        on an archived ticket's id."""
        # Seed: create -01 and archive it.
        make_ticket(self.env, "numbering-01")
        run_close(self.env, "numbering-01")
        archive = self.env.pipeline / "tickets" / "archive"
        self.assertTrue((archive / "numbering-01.md").exists())

        # Next merged consolidation using the same stem base should take
        # -02, not reuse -01. Exercise via consolidate (easier than a
        # full cmd_audit) — the same all_ticket_paths helper drives both.
        import json as _json
        make_ticket(self.env, "src-01", slugs=["s"])
        staging = self.env.pipeline / "staging" / "consolidation-numbering.json"
        staging.write_text(_json.dumps({
            "slug": "numbering",
            "title": "Numbering",
            "description": "d",
            "engine_path": ["x.rs:1"],
            "tests": [{"slug": "s", "source_ticket": "src-01",
                       "scenario": "e"}],
        }, indent=2))
        cli.cmd_consolidate(argparse.Namespace(
            input=str(staging), keep_input=False, dry_run=False))
        # merged-numbering-01 was a prior ticket (archived) so new id is -02
        self.assertTrue((self.env.pipeline / "tickets" / "archive" / "numbering-01.md").exists())
        self.assertTrue((self.env.pipeline / "tickets" / "merged-numbering-01.md").exists())
        # Now create a second consolidation using the same slug: must get -02
        make_ticket(self.env, "src-02", slugs=["s2"])
        staging2 = self.env.pipeline / "staging" / "consolidation-numbering2.json"
        staging2.write_text(_json.dumps({
            "slug": "numbering",
            "title": "Numbering 2",
            "description": "d",
            "engine_path": ["x.rs:1"],
            "tests": [{"slug": "s2", "source_ticket": "src-02",
                       "scenario": "e"}],
        }, indent=2))
        cli.cmd_consolidate(argparse.Namespace(
            input=str(staging2), keep_input=False, dry_run=False))
        self.assertTrue((self.env.pipeline / "tickets" / "merged-numbering-02.md").exists())
        # Ship the first parent so it moves to archive, then do a third
        # consolidation — must take -03, not reuse -01 or -02.
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        run_test(self.env, "merged-numbering-01")
        run_fix(self.env, "merged-numbering-01")
        run_merge(self.env, "merged-numbering-01")
        self.assertEqual(self.env.status("merged-numbering-01"), cli.STATUS_SHIPPED)
        self.assertTrue((self.env.pipeline / "tickets" / "archive" / "merged-numbering-01.md").exists())

        make_ticket(self.env, "src-03", slugs=["s3"])
        staging3 = self.env.pipeline / "staging" / "consolidation-numbering3.json"
        staging3.write_text(_json.dumps({
            "slug": "numbering",
            "title": "Numbering 3",
            "description": "d",
            "engine_path": ["x.rs:1"],
            "tests": [{"slug": "s3", "source_ticket": "src-03",
                       "scenario": "e"}],
        }, indent=2))
        cli.cmd_consolidate(argparse.Namespace(
            input=str(staging3), keep_input=False, dry_run=False))
        self.assertTrue((self.env.pipeline / "tickets" / "merged-numbering-03.md").exists())

    def test_list_tickets_spans_active_and_archive(self) -> None:
        """list_tickets reads both directories so reports/backlog
        queries see the full history."""
        make_ticket(self.env, "live-01")
        make_ticket(self.env, "dead-01")
        run_close(self.env, "dead-01")

        all_tickets = cli.list_tickets()
        ids = {t["frontmatter"].get("id") for t in all_tickets}
        self.assertIn("live-01", ids)
        self.assertIn("dead-01", ids)

        # Filtering by status also sees the archived ones
        closed = cli.list_tickets(status=cli.STATUS_CLOSED)
        self.assertEqual({t["frontmatter"]["id"] for t in closed}, {"dead-01"})
        new = cli.list_tickets(status=cli.STATUS_NEW)
        self.assertEqual({t["frontmatter"]["id"] for t in new}, {"live-01"})

    def test_absorb_tested_source_inherits_worktree(self) -> None:
        """Absorbing a single `tested` merged-* source into a deeper new
        parent: the worktree and branch are renamed, the test file is
        renamed on disk and committed, and inherited Implementation
        pointers appear in the new parent's body (pointing at the new
        test-file name). If all tests inherit, the new parent goes
        straight to status=tested."""
        # Set up: create a merged parent via consolidate, then run test
        # on it so its status becomes `tested`.
        for tid, slug in (("a-01", "a_slug"), ("b-01", "b_slug")):
            make_ticket(self.env, tid, slugs=[slug])
        import json as _json
        stg = self.env.pipeline / "staging" / "consolidation-first.json"
        stg.write_text(_json.dumps({
            "slug": "first", "title": "First", "description": "d",
            "engine_path": ["e.rs:1"],
            "tests": [
                {"slug": "a_slug", "source_ticket": "a-01", "scenario": "s"},
                {"slug": "b_slug", "source_ticket": "b-01", "scenario": "s"},
            ],
        }, indent=2))
        cli.cmd_consolidate(argparse.Namespace(
            input=str(stg), keep_input=False, dry_run=False))
        parent1 = "merged-first-01"
        self.assertEqual(self.env.status(parent1), cli.STATUS_NEW)

        # Test-writer runs on parent1 → status: tested
        self.env.install_agent([tester_confirms()])
        run_test(self.env, parent1)
        self.assertEqual(self.env.status(parent1), cli.STATUS_TESTED)
        parent1_test_file = (self.env.read_ticket(parent1)
                             ["frontmatter"]["test_file"])
        self.assertTrue(parent1_test_file.endswith(
            "pipeline_bugs_merged_first_01.rs"))

        # Now dedup: absorb parent1 (via Also closes) + a new open card
        # ticket into a deeper parent2. The new parent's tests should
        # include copies of parent1's slugs verbatim PLUS a new slug
        # for the card ticket.
        make_ticket(self.env, "c-01", slugs=["c_slug"])
        stg2 = self.env.pipeline / "staging" / "consolidation-deep.json"
        stg2.write_text(_json.dumps({
            "slug": "deep", "title": "Deep", "description": "d",
            "engine_path": ["e.rs:1"],
            "tests": [
                # Copies verbatim from parent1 (slugs preserved)
                {"slug": "a_slug", "source_ticket": "a-01", "scenario": "s"},
                {"slug": "b_slug", "source_ticket": "b-01", "scenario": "s"},
                # New entry for the card ticket
                {"slug": "c_slug", "source_ticket": "c-01", "scenario": "s"},
            ],
            "also_closes": [parent1],
        }, indent=2))
        cli.cmd_consolidate(argparse.Namespace(
            input=str(stg2), keep_input=False, dry_run=False))

        parent2 = "merged-deep-01"
        # parent1 archived
        self.assertEqual(self.env.status(parent1), cli.STATUS_CLOSED)
        # parent2 has mixed implementations — a_slug & b_slug inherited,
        # c_slug still needs implementing. So status stays `new`.
        self.assertEqual(self.env.status(parent2), cli.STATUS_NEW)
        fm2 = self.env.read_ticket(parent2)["frontmatter"]
        self.assertEqual(fm2.get("inherited_from"), parent1)
        new_test_file = fm2["test_file"]
        self.assertTrue(new_test_file.endswith(
            "pipeline_bugs_merged_deep_01.rs"))

        # Check body has inherited Implementation pointers on a/b and
        # (not yet written) on c, all pointing at the NEW test file.
        body2 = self.env.read_ticket(parent2)["body"]
        self.assertIn(f"Implementation: {new_test_file}::test_a_slug", body2)
        self.assertIn(f"Implementation: {new_test_file}::test_b_slug", body2)
        self.assertIn("Implementation: (not yet written)", body2)

        # The worktree was renamed: parent1's dir is gone, parent2's exists.
        self.assertFalse(cli.get_worktree_dir(parent1).exists())
        self.assertTrue(cli.get_worktree_dir(parent2).exists())
        # The test file was renamed on disk.
        wt = cli.get_worktree_dir(parent2)
        self.assertTrue((wt / new_test_file).exists())
        self.assertFalse((wt / parent1_test_file).exists())

    def test_absorb_rejects_multiple_tested(self) -> None:
        """Consolidation may inherit from at most one tested source."""
        # Build two separately-tested merged parents.
        for slug in ("x", "y"):
            make_ticket(self.env, f"{slug}-01", slugs=[f"{slug}_slug"])
        import json as _json
        for slug in ("x", "y"):
            stg = self.env.pipeline / "staging" / f"cons-{slug}.json"
            stg.write_text(_json.dumps({
                "slug": slug, "title": slug, "description": "d",
                "engine_path": ["e.rs:1"],
                "tests": [{"slug": f"{slug}_slug",
                           "source_ticket": f"{slug}-01", "scenario": "s"}],
            }, indent=2))
            cli.cmd_consolidate(argparse.Namespace(
                input=str(stg), keep_input=False, dry_run=False))
        self.env.install_agent([tester_confirms(), tester_confirms()])
        run_test(self.env, "merged-x-01")
        run_test(self.env, "merged-y-01")
        self.assertEqual(self.env.status("merged-x-01"), cli.STATUS_TESTED)
        self.assertEqual(self.env.status("merged-y-01"), cli.STATUS_TESTED)

        # Now attempt to absorb both tested parents into one deeper parent.
        stg3 = self.env.pipeline / "staging" / "cons-both.json"
        stg3.write_text(_json.dumps({
            "slug": "both", "title": "Both", "description": "d",
            "engine_path": ["e.rs:1"],
            "tests": [
                {"slug": "x_slug", "source_ticket": "x-01", "scenario": "s"},
                {"slug": "y_slug", "source_ticket": "y-01", "scenario": "s"},
            ],
            "also_closes": ["merged-x-01", "merged-y-01"],
        }, indent=2))
        with self.assertRaises(SystemExit):
            cli.cmd_consolidate(argparse.Namespace(
                input=str(stg3), keep_input=False, dry_run=False))
        # Neither parent is re-absorbed; both still tested.
        self.assertEqual(self.env.status("merged-x-01"), cli.STATUS_TESTED)
        self.assertEqual(self.env.status("merged-y-01"), cli.STATUS_TESTED)
        self.assertFalse(
            (self.env.pipeline / "tickets" / "merged-both-01.md").exists())

    def test_absorb_rejects_fixed_or_fix_failed(self) -> None:
        """Only `new` and `tested` sources may be absorbed. `fixed` and
        `fix_failed` carry real work (commits, post-mortem) and are
        off-limits — the user must retry --to new first."""
        # Set up: a fixed ticket and a fix_failed ticket.
        make_ticket(self.env, "fx-01", slugs=["fx_slug"])
        self.env.install_agent([tester_confirms(), fixer_succeeds()])
        run_test(self.env, "fx-01")
        run_fix(self.env, "fx-01")
        self.assertEqual(self.env.status("fx-01"), cli.STATUS_FIXED)

        make_ticket(self.env, "ff-01", slugs=["ff_slug"])
        self.env.install_agent([tester_confirms(), fixer_fails()])
        run_test(self.env, "ff-01")
        run_fix(self.env, "ff-01")
        self.assertEqual(self.env.status("ff-01"), cli.STATUS_FIX_FAILED)

        # Try to absorb each into a new parent — each should be rejected.
        import json as _json
        for source in ("fx-01", "ff-01"):
            stg = self.env.pipeline / "staging" / f"cons-{source}.json"
            stg.write_text(_json.dumps({
                "slug": source.replace("-", ""),
                "title": "T", "description": "d",
                "engine_path": ["e.rs:1"],
                "tests": [{"slug": f"{source.replace('-', '_')}_slug",
                           "source_ticket": source, "scenario": "s"}],
            }, indent=2))
            with self.assertRaises(SystemExit):
                cli.cmd_consolidate(argparse.Namespace(
                    input=str(stg), keep_input=False, dry_run=False))
        # Both sources unchanged.
        self.assertEqual(self.env.status("fx-01"), cli.STATUS_FIXED)
        self.assertEqual(self.env.status("ff-01"), cli.STATUS_FIX_FAILED)

    # J10: Status guards — test refuses non-new, fix refuses non-tested.
    def test_j10_status_guards(self) -> None:
        tid = "guard-01"
        make_ticket(self.env, tid)
        self.env.install_agent([tester_confirms()])
        run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_TESTED)

        # `test` on a tested ticket is a no-op: cmd_test filters out
        # any ticket whose status != new and prints a message.
        run_test(self.env, tid)
        self.assertEqual(self.env.status(tid), cli.STATUS_TESTED)

        # `fix` on a new ticket is blocked with a clear error.
        tid2 = "guard-02"
        make_ticket(self.env, tid2)
        with self.assertRaises(SystemExit):
            run_fix(self.env, tid2)
        self.assertEqual(self.env.status(tid2), cli.STATUS_NEW)


if __name__ == "__main__":
    unittest.main(verbosity=2)
