r"""Integration test for sandbox profiles via `@anthropic-ai/sandbox-runtime`.

Spawns the actual sandbox runtime (`srt`) with the `filesystem` block
extracted from each rendered profile and tries to perform a denied write
inside the sandbox. The kernel — Seatbelt on macOS, bubblewrap on
Linux — should reject the write with EPERM, so the file never appears.

This is the only thing in the suite that proves the deny patterns are
actually enforced (vs. just being syntactically valid JSON).

## Auto-skip

`srt` is heavy (npx + npm cache populate on first call can be 30s+),
so we only run when it's already locally available and responds within
2 seconds. If the precheck fails the entire class is skipped silently —
keeps the regular `python -m unittest` flow under a few seconds.

To force-run after first install:
    RUN_SANDBOX_INTEGRATION=1 python -m unittest \\
        new_pipeline.tests.test_sandbox_integration

To prime the cache so future runs auto-include this:
    npx -y @anthropic-ai/sandbox-runtime --version

## Profile → SRT format adaptation

Our profiles target Claude Code's settings.json shape
(`sandbox.filesystem.*` + `sandbox.enabled`). SRT uses `filesystem.*`
at the top level. We extract `cfg["sandbox"]["filesystem"]` and add
`"."` to `allowWrite` to mimic Claude's implicit cwd-default-allow
(SRT defaults to deny-all-writes; Claude defaults to allow-cwd).
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import sandbox  # noqa: E402

_SRT_PRECHECK_TIMEOUT = 2.0  # seconds — fast-fail if not cached


def _srt_command() -> list[str] | None:
    """Return the cmd prefix to invoke srt, or None if not fast-available.

    Order: prefer a directly-installed `srt` on PATH (fastest), fall
    back to `npx --no-install @anthropic-ai/sandbox-runtime`. Either
    way, must respond to `--version` within `_SRT_PRECHECK_TIMEOUT`.
    """
    candidates: list[list[str]] = []
    if shutil.which("srt") is not None:
        candidates.append(["srt"])
    if shutil.which("npx") is not None:
        candidates.append(
            ["npx", "--no-install", "@anthropic-ai/sandbox-runtime"]
        )
    for cmd in candidates:
        try:
            r = subprocess.run(
                [*cmd, "--version"],
                capture_output=True,
                timeout=_SRT_PRECHECK_TIMEOUT,
            )
        except (subprocess.TimeoutExpired, FileNotFoundError):
            continue
        if r.returncode == 0:
            return cmd
    return None


def _should_run() -> tuple[bool, str]:
    """Decide whether to run the suite. Returns (run, skip_reason)."""
    if os.environ.get("RUN_SANDBOX_INTEGRATION") == "0":
        return False, "RUN_SANDBOX_INTEGRATION=0 explicitly opted out"
    if _srt_command() is None:
        return False, (
            "@anthropic-ai/sandbox-runtime not locally available within "
            f"{_SRT_PRECHECK_TIMEOUT}s; prime with "
            "`npx -y @anthropic-ai/sandbox-runtime --version` to enable"
        )
    return True, ""


_RUN, _SKIP_REASON = _should_run()


def _to_srt_settings(profile_json: str) -> dict:
    """Strip Claude's `sandbox.` wrapper; keep filesystem.* at root.

    Also adds `"."` to `allowWrite` so the sandbox-default `cwd is
    writable` matches what Claude assumes — without it, SRT would
    deny every write (it's allow-only by default), which isn't what
    our profiles model.
    """
    cfg = json.loads(profile_json)
    fs = cfg["sandbox"]["filesystem"]
    fs.setdefault("allowWrite", []).insert(0, ".")
    return {"filesystem": fs}


@unittest.skipUnless(_RUN, _SKIP_REASON)
class SandboxIntegrationTest(unittest.TestCase):
    """Real sandbox: a denied write must not produce a file."""

    @classmethod
    def setUpClass(cls) -> None:
        """Cache the discovered srt cmd prefix for the per-test runs."""
        cls.srt_cmd = _srt_command()
        assert cls.srt_cmd is not None  # guaranteed by skipUnless

    def _run_in_sandbox(
        self, profile: str, shell_cmd: str, repo: Path,
    ) -> subprocess.CompletedProcess:
        """Render `profile`, write its filesystem rules, run shell_cmd."""
        rendered = sandbox.render(profile, project_root=str(repo))
        srt_settings = _to_srt_settings(rendered)
        settings_path = repo / ".srt-settings.json"
        settings_path.write_text(json.dumps(srt_settings))
        return subprocess.run(
            [*self.srt_cmd, "--settings", str(settings_path), shell_cmd],
            cwd=str(repo),
            capture_output=True,
            text=True,
            timeout=10,
        )

    def test_fixer_blocks_test_file_write(self) -> None:
        """Fixer profile: writing to `mtg-engine/tests/` must be denied."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / "mtg-engine" / "tests").mkdir(parents=True)
            target = repo / "mtg-engine" / "tests" / "leaked.rs"
            r = self._run_in_sandbox(
                "fixer", f'echo "x" > {target}', repo,
            )
            self.assertFalse(
                target.exists(),
                f"deny rule failed; srt stderr:\n{r.stderr}",
            )

    def test_test_writer_blocks_engine_src_write(self) -> None:
        """test_writer profile: `mtg-engine/src/` must be denied."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / "mtg-engine" / "src").mkdir(parents=True)
            target = repo / "mtg-engine" / "src" / "leaked.rs"
            r = self._run_in_sandbox(
                "test_writer", f'echo "x" > {target}', repo,
            )
            self.assertFalse(
                target.exists(),
                f"deny rule failed; srt stderr:\n{r.stderr}",
            )

    def test_test_writer_engine_permits_engine_src_write(self) -> None:
        """test_writer_engine relaxation: engine src write must succeed.

        This is the inverse check — if this test starts failing, our
        retry-after-needs_engine_work flow can't actually do its job.
        """
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / "mtg-engine" / "src").mkdir(parents=True)
            target = repo / "mtg-engine" / "src" / "patch.rs"
            r = self._run_in_sandbox(
                "test_writer_engine", f'echo "ok" > {target}', repo,
            )
            self.assertTrue(
                target.exists(),
                f"engine src write was blocked; srt stderr:\n{r.stderr}",
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
