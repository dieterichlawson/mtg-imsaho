"""Tests for `run_agent` — the one-shot claude subprocess wrapper.

`subprocess.Popen` is mocked with a small fake that emits canned
stream-json lines, so these tests never invoke the real `claude` CLI.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import agent  # noqa: E402
from new_pipeline.agent import AgentResult, run_agent  # noqa: E402


class _FakePopen:
    """Minimal stand-in for subprocess.Popen.

    Emits lines from `stdout_lines` one at a time as stdout iteration.
    Records the env it was constructed with so tests can inspect it.
    """

    last_env: dict[str, str] | None = None  # most-recent constructor env
    last_cmd: list[str] | None = None  # most-recent constructor cmd

    def __init__(
        self,
        cmd,
        *,
        stdout,
        stderr,
        text,
        cwd,
        env,
        bufsize,
        stdout_lines=(),
        stderr_output="",
        returncode=0,
        timeout_on_wait=False,
    ):
        self.cmd = cmd
        self.stdout = iter(stdout_lines)
        self.stderr = _FakeReader(stderr_output)
        self._returncode = returncode
        self._timeout_on_wait = timeout_on_wait
        self.killed = False
        _FakePopen.last_env = env
        _FakePopen.last_cmd = list(cmd)

    def wait(self, timeout=None):
        if self._timeout_on_wait and not self.killed:
            import subprocess as _sp
            raise _sp.TimeoutExpired(cmd=self.cmd, timeout=timeout)
        return self._returncode

    def kill(self):
        self.killed = True


class _FakeReader:
    def __init__(self, s: str) -> None:
        self._s = s

    def read(self) -> str:
        return self._s


def _popen_factory(**fake_kwargs):
    """Build a Popen stand-in that injects the given fake kwargs."""

    def _factory(cmd, **popen_kwargs):
        return _FakePopen(cmd, **popen_kwargs, **fake_kwargs)

    return _factory


def _event(kind: str, **fields) -> str:
    """One JSON-line for the fake stdout."""
    return json.dumps({"type": kind, **fields}) + "\n"


class RunAgentTest(unittest.TestCase):
    """Exercise the happy path, the error path, and timeout handling."""

    def test_happy_path_populates_tokens_and_turns(self) -> None:
        """A result event with usage → AgentResult carries those numbers."""
        stdout_lines = [
            _event("assistant"),
            _event(
                "result",
                usage={"input_tokens": 100, "output_tokens": 40},
                num_turns=3,
            ),
        ]
        with patch.object(
            agent.subprocess, "Popen",
            _popen_factory(stdout_lines=stdout_lines, returncode=0),
        ):
            res = run_agent(
                "hello", cwd=Path("."), model="opus", effort="max",
            )
        self.assertIsInstance(res, AgentResult)
        self.assertEqual(res.tokens, 140)
        self.assertEqual(res.tool_uses, 3)
        self.assertFalse(res.is_error)
        self.assertIsNone(res.error_message)

    def test_agent_reported_error_passes_through(self) -> None:
        """`result` event with is_error=true sets AgentResult.is_error."""
        stdout_lines = [
            _event(
                "result",
                usage={"input_tokens": 1, "output_tokens": 1},
                num_turns=1,
                is_error=True,
                result="the agent explains what broke",
            ),
        ]
        with patch.object(
            agent.subprocess, "Popen",
            _popen_factory(stdout_lines=stdout_lines, returncode=0),
        ):
            res = run_agent(
                "hi", cwd=Path("."), model="opus", effort="max",
            )
        self.assertTrue(res.is_error)
        self.assertEqual(res.error_message, "the agent explains what broke")

    def test_nonzero_exit_surfaces_stderr(self) -> None:
        """Non-zero exit → is_error=True with the stderr tail as message."""
        with patch.object(
            agent.subprocess, "Popen",
            _popen_factory(
                stdout_lines=[],
                stderr_output="claude crashed: something",
                returncode=2,
            ),
        ):
            res = run_agent(
                "hi", cwd=Path("."), model="opus", effort="max",
            )
        self.assertTrue(res.is_error)
        self.assertEqual(res.error_message, "claude crashed: something")

    def test_timeout_kills_and_reports(self) -> None:
        """proc.wait raising TimeoutExpired → kill + is_error set."""
        with patch.object(
            agent.subprocess, "Popen",
            _popen_factory(
                stdout_lines=[],
                timeout_on_wait=True,
                returncode=0,
            ),
        ):
            res = run_agent(
                "hi",
                cwd=Path("."),
                model="opus",
                effort="max",
                timeout_secs=1,
            )
        self.assertTrue(res.is_error)
        assert res.error_message is not None
        self.assertIn("timeout", res.error_message)

    def test_env_scrubs_api_key_vars(self) -> None:
        """Subprocess env drops ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN."""
        with patch.dict(
            "os.environ",
            {
                "ANTHROPIC_API_KEY": "secret-key",
                "ANTHROPIC_AUTH_TOKEN": "secret-token",
                "PATH": "/usr/bin",
            },
            clear=True,
        ), patch.object(
            agent.subprocess, "Popen",
            _popen_factory(stdout_lines=[], returncode=0),
        ):
            run_agent("hi", cwd=Path("."), model="opus", effort="max")
        env = _FakePopen.last_env
        assert env is not None
        self.assertNotIn("ANTHROPIC_API_KEY", env)
        self.assertNotIn("ANTHROPIC_AUTH_TOKEN", env)
        self.assertEqual(env.get("PATH"), "/usr/bin")


    def test_no_sandbox_runs_claude_directly(self) -> None:
        """No sandbox kwarg → cmd starts with `claude`, not `srt`."""
        with patch.object(
            agent.subprocess, "Popen",
            _popen_factory(stdout_lines=[], returncode=0),
        ):
            run_agent("hi", cwd=Path("."), model="opus", effort="max")
        cmd = _FakePopen.last_cmd
        assert cmd is not None
        self.assertEqual(cmd[0], "claude")
        self.assertNotIn("srt", cmd)

    def test_sandbox_wraps_in_srt(self) -> None:
        """sandbox_settings_path → cmd is `srt --settings <path> -c <claude>`.

        srt is invoked via the `-c '<shell-string>'` form because its
        positional-argv form mangles multi-word arguments (the prompt
        comes back to claude truncated). The full claude invocation is
        shell-quoted and passed as one arg.
        """
        settings_path = Path("/tmp/test-sandbox.json")
        with patch.object(
            agent.subprocess, "Popen",
            _popen_factory(stdout_lines=[], returncode=0),
        ):
            run_agent(
                "hi",
                cwd=Path("."),
                model="opus",
                effort="max",
                sandbox_settings_path=settings_path,
            )
        cmd = _FakePopen.last_cmd
        assert cmd is not None
        # First three args: srt --settings <path>
        self.assertEqual(cmd[0], "srt")
        self.assertEqual(cmd[1], "--settings")
        self.assertEqual(cmd[2], str(settings_path))
        # Fourth: -c, fifth: a single shell string starting with `claude `
        self.assertEqual(cmd[3], "-c")
        self.assertTrue(cmd[4].startswith("claude "))
        # The prompt and other args are inside the shell string, quoted.
        self.assertIn("hi", cmd[4])
        self.assertIn("--model", cmd[4])
        self.assertIn("opus", cmd[4])


if __name__ == "__main__":
    unittest.main(verbosity=2)
