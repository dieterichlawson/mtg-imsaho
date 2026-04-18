"""Integration test for sandbox profiles via `@anthropic-ai/sandbox-runtime`.

Spawns the actual sandbox runtime (`srt`) with each rendered profile
and tries to perform a write that should be allowed and a write that
should be denied. The kernel — Seatbelt on macOS, bubblewrap on Linux
— enforces the rules; a denied write returns EPERM and the file never
appears.

This is the only thing in the suite that proves the deny patterns are
actually enforced (vs. just being syntactically valid JSON).

## Auto-skip

`srt` is heavy enough that we only run when it's locally installed
and responds within 2 seconds. If the precheck fails the entire class
is skipped silently — keeps the regular `python -m unittest` flow
under a few seconds.

To install:
    brew install node
    npm install -g @anthropic-ai/sandbox-runtime

Or via npx (slower; cache populates on first call):
    npx -y @anthropic-ai/sandbox-runtime --version
"""

from __future__ import annotations

import json
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


_SRT_CMD = _srt_command()


@unittest.skipUnless(
    _SRT_CMD is not None,
    "srt not locally available within "
    f"{_SRT_PRECHECK_TIMEOUT}s; install via "
    "`npm install -g @anthropic-ai/sandbox-runtime` to enable",
)
class SandboxIntegrationTest(unittest.TestCase):
    """Real sandbox: a denied write must not produce a file."""

    def _write_settings(
        self, repo: Path, profile: str,
    ) -> Path:
        """Render a profile (test-adjusted), write to disk, return path.

        Strips `/tmp` and `/var/folders` from `allowWrite` because
        macOS tempfile.TemporaryDirectory creates the test repo
        *under* `/var/folders`. With those entries present, every
        path inside the test dir is implicitly allowed and the deny
        assertions pass trivially. Production runs need them (cargo
        + tools spill into tmp), but the test doesn't.
        """
        cfg = json.loads(sandbox.render(
            profile,
            project_root=str(repo),
            home=str(repo / "fake-home"),
            test_file="mtg-engine/tests/pipeline_bugs_foo_01.rs",
        ))
        cfg["filesystem"]["allowWrite"] = [
            p for p in cfg["filesystem"]["allowWrite"]
            if p not in ("/tmp", "/var/folders")
        ]
        path = repo / f"{profile}.srt.json"
        path.write_text(json.dumps(cfg))
        return path

    def _try_write(
        self, settings_path: Path, repo: Path, target_rel: str,
    ) -> bool:
        """Run `srt -c 'echo > <target>'` and return True if file appears."""
        target = repo / target_rel
        target.parent.mkdir(parents=True, exist_ok=True)
        if target.exists():
            target.unlink()
        subprocess.run(
            [
                *_SRT_CMD,
                "--settings", str(settings_path),
                "-c", f'echo "x" > {target}',
            ],
            cwd=str(repo),
            capture_output=True,
            timeout=10,
        )
        return target.exists()

    def test_fixer_blocks_test_file_write(self) -> None:
        """Fixer profile: writing to `mtg-engine/tests/` must be denied."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            settings = self._write_settings(repo, "fixer")
            self.assertFalse(
                self._try_write(
                    settings, repo, "mtg-engine/tests/leaked.rs",
                ),
                "fixer should not be able to write to mtg-engine/tests/",
            )

    def test_fixer_permits_engine_src_write(self) -> None:
        """Fixer profile: writing to `mtg-engine/src/` must succeed."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            settings = self._write_settings(repo, "fixer")
            self.assertTrue(
                self._try_write(settings, repo, "mtg-engine/src/patch.rs"),
                "fixer should be able to write to mtg-engine/src/",
            )

    def test_test_writer_blocks_engine_src_write(self) -> None:
        """test_writer profile: `mtg-engine/src/` must be denied."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            settings = self._write_settings(repo, "test_writer")
            self.assertFalse(
                self._try_write(
                    settings, repo, "mtg-engine/src/leaked.rs",
                ),
                "default test_writer must not write engine src",
            )

    def test_test_writer_locks_to_single_test_file(self) -> None:
        """test_writer can write its specific test file but NOT a sibling.

        The sandbox is locked to `./<test_file>` (the exact path passed
        in via the template var), not the whole `mtg-engine/tests/` dir,
        so the agent can't drop a stray test file or overwrite an
        unrelated existing test.
        """
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            settings = self._write_settings(repo, "test_writer")
            # The configured file is allowed.
            self.assertTrue(
                self._try_write(
                    settings, repo,
                    "mtg-engine/tests/pipeline_bugs_foo_01.rs",
                ),
                "test_writer must be able to write its configured test file",
            )
            # A sibling test file is blocked.
            self.assertFalse(
                self._try_write(
                    settings, repo,
                    "mtg-engine/tests/some_other_test.rs",
                ),
                "test_writer must NOT be able to write a sibling test file",
            )

    def test_test_writer_engine_permits_engine_src_write(self) -> None:
        """test_writer_engine relaxation: engine src write must succeed."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            settings = self._write_settings(repo, "test_writer_engine")
            self.assertTrue(
                self._try_write(
                    settings, repo, "mtg-engine/src/patch.rs",
                ),
                "test_writer_engine retry must be able to write engine src",
            )

    def test_auditor_blocks_engine_writes(self) -> None:
        """Auditor profile: `mtg-engine/**` writes denied."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            settings = self._write_settings(repo, "auditor")
            self.assertFalse(
                self._try_write(
                    settings, repo, "mtg-engine/src/leaked.rs",
                ),
                "auditor must not write to engine",
            )

    def test_auditor_blocks_ticket_write(self) -> None:
        """Auditor profile: ticket writes denied (Python mints tickets).

        The auditor agent only emits its report to staging. The actual
        ticket files are written by `AuditReport.mint_tickets()` in
        the main Python process after the agent finishes, outside the
        sandbox. So if the AGENT itself tries to write a ticket file,
        that's unexpected behavior and should be denied.
        """
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            settings = self._write_settings(repo, "auditor")
            self.assertFalse(
                self._try_write(
                    settings, repo, "new_pipeline/tickets/foo-01.md",
                ),
                "auditor agent should not write tickets directly",
            )

    def test_auditor_permits_staging_write(self) -> None:
        """Auditor profile: staging report write succeeds."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            settings = self._write_settings(repo, "auditor")
            self.assertTrue(
                self._try_write(
                    settings, repo, "new_pipeline/staging/run-id.json",
                ),
                "auditor must be able to write its report to staging",
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
