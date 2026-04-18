"""Tests for `run_agent_loop` — the retry wrapper around `run_agent`.

`run_agent` is already covered by `test_agent.py`; here it's mocked
with a scripted list of canned `AgentResult`s so we can exercise the
retry logic in isolation.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import agent  # noqa: E402
from new_pipeline.agent import AgentResult, run_agent_loop  # noqa: E402


def _ok(tokens: int = 10) -> AgentResult:
    """AgentResult with no error — run_agent reported success."""
    return AgentResult(
        tokens=tokens,
        tool_uses=1,
        duration=1,
        is_error=False,
        error_message=None,
    )


def _err(msg: str = "agent exploded") -> AgentResult:
    """AgentResult with is_error=True — run_agent reported a failure."""
    return AgentResult(
        tokens=0,
        tool_uses=0,
        duration=0,
        is_error=True,
        error_message=msg,
    )


class _Recorder:
    """Records the prompts handed to each attempt and the attempt number."""

    def __init__(self) -> None:
        self.prompts: list[str] = []

    def build_prompt(self, retry_note: str, attempt: int) -> str:
        prompt = f"ATTEMPT={attempt}{retry_note}"
        self.prompts.append(prompt)
        return prompt


def _scripted_run_agent(results: list[AgentResult]):
    """Return a drop-in for run_agent that pops from `results` per call."""
    it = iter(results)

    def _fn(prompt, *, cwd, model, effort, settings=None, timeout_secs=3600):
        return next(it)

    return _fn


class RunAgentLoopTest(unittest.TestCase):
    """Retry loop: retry notes, final-attempt stamping, invariants."""

    def test_accept_on_first_attempt(self) -> None:
        """Loader returns (parsed, None) on attempt 1 → one run_agent call."""
        rec = _Recorder()
        agent_results = [_ok()]

        def loader(result, attempt):
            return {"payload": "ok"}, None

        with patch.object(
            agent, "run_agent", _scripted_run_agent(agent_results),
        ):
            parsed, result = run_agent_loop(
                build_prompt=rec.build_prompt,
                cwd=Path("."),
                load_result=loader,
                model="opus",
                effort="max",
                max_attempts=3,
            )

        self.assertEqual(parsed, {"payload": "ok"})
        self.assertFalse(result.is_error)
        self.assertEqual(len(rec.prompts), 1)
        self.assertEqual(rec.prompts[0], "ATTEMPT=1")

    def test_retries_carry_previous_error_in_prompt(self) -> None:
        """Each retry's prompt embeds the previous attempt's failure reason."""
        rec = _Recorder()
        agent_results = [_ok(), _ok(), _ok()]
        attempts_seen: list[int] = []

        def loader(result, attempt):
            attempts_seen.append(attempt)
            if attempt < 3:
                return None, f"bad output from attempt {attempt}"
            return "finally!", None

        with patch.object(
            agent, "run_agent", _scripted_run_agent(agent_results),
        ):
            parsed, result = run_agent_loop(
                build_prompt=rec.build_prompt,
                cwd=Path("."),
                load_result=loader,
                model="opus",
                effort="max",
                max_attempts=3,
            )

        self.assertEqual(parsed, "finally!")
        self.assertFalse(result.is_error)
        self.assertEqual(attempts_seen, [1, 2, 3])
        # Attempt 2's prompt carries attempt 1's reason; attempt 3 carries 2's.
        self.assertIn("bad output from attempt 1", rec.prompts[1])
        self.assertIn("bad output from attempt 2", rec.prompts[2])
        # Attempt 1 has no retry note yet.
        self.assertNotIn("Retry note", rec.prompts[0])

    def test_all_attempts_fail_stamps_is_error(self) -> None:
        """If no attempt succeeds, the final result has is_error=True set."""
        rec = _Recorder()
        agent_results = [_ok(), _ok()]  # agent succeeds, loader rejects

        def loader(result, attempt):
            return None, "still bad"

        with patch.object(
            agent, "run_agent", _scripted_run_agent(agent_results),
        ):
            parsed, result = run_agent_loop(
                build_prompt=rec.build_prompt,
                cwd=Path("."),
                load_result=loader,
                model="opus",
                effort="max",
                max_attempts=2,
            )

        self.assertIsNone(parsed)
        self.assertTrue(result.is_error)
        self.assertEqual(result.error_message, "still bad")

    def test_agent_error_still_retries(self) -> None:
        """An is_error=True result still goes through loader (so loader can.

        decide whether to retry based on it). Then retries proceed normally.
        """
        rec = _Recorder()
        agent_results = [_err("timeout"), _ok()]
        seen_errors: list[bool] = []

        def loader(result, attempt):
            seen_errors.append(result.is_error)
            if result.is_error:
                return None, f"retry after: {result.error_message}"
            return "success-after-agent-error", None

        with patch.object(
            agent, "run_agent", _scripted_run_agent(agent_results),
        ):
            parsed, result = run_agent_loop(
                build_prompt=rec.build_prompt,
                cwd=Path("."),
                load_result=loader,
                model="opus",
                effort="max",
                max_attempts=2,
            )

        self.assertEqual(parsed, "success-after-agent-error")
        self.assertFalse(result.is_error)
        self.assertEqual(seen_errors, [True, False])
        self.assertIn("retry after: timeout", rec.prompts[1])

    def test_postcondition_parsed_iff_is_error(self) -> None:
        """Enforces: parsed is None ⟺ result.is_error is True."""
        # Case A: success → parsed non-None, is_error=False.
        with patch.object(agent, "run_agent", _scripted_run_agent([_ok()])):
            parsed, result = run_agent_loop(
                build_prompt=lambda _note, _att: "x",
                cwd=Path("."),
                load_result=lambda _r, _a: ("x", None),
                model="opus",
                effort="max",
                max_attempts=1,
            )
        self.assertIsNotNone(parsed)
        self.assertFalse(result.is_error)

        # Case B: exhausted → parsed=None, is_error=True (stamped).
        with patch.object(agent, "run_agent", _scripted_run_agent([_ok()])):
            parsed, result = run_agent_loop(
                build_prompt=lambda _note, _att: "x",
                cwd=Path("."),
                load_result=lambda _r, _a: (None, "nope"),
                model="opus",
                effort="max",
                max_attempts=1,
            )
        self.assertIsNone(parsed)
        self.assertTrue(result.is_error)


if __name__ == "__main__":
    unittest.main(verbosity=2)
