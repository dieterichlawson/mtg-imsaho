"""Spawn the `claude` CLI in a subprocess and collect a typed result.

One function — `run_agent(prompt, cwd, ...)` — wraps a single invocation
of the `claude` binary, streams its stream-json events, and returns an
`AgentResult` carrying tokens, duration, and any error. No retry logic;
no log-file writing; no progress printing. Callers layer those on later.

The child process is run with `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN`
scrubbed from its environment so it falls back to subscription auth.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

# Env vars that force API-key billing when set. Scrubbed from agent
# subprocesses so `claude` picks subscription auth.
_API_KEY_ENV_VARS = ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN")


@dataclass
class AgentResult:
    """What came out of one `run_agent` call."""

    tokens: int
    tool_uses: int
    duration: int  # wall-clock seconds
    is_error: bool
    error_message: str | None


def run_agent(
    prompt: str,
    *,
    cwd: Path,
    model: str,
    effort: str,
    timeout_secs: int = 3600,
) -> AgentResult:
    """Run `claude -p <prompt>` in `cwd`, collect usage + errors, return them.

    The child is killed if it runs past `timeout_secs`, and the returned
    `AgentResult` has `is_error=True` with a timeout message in that case.
    Agent-reported errors (the `result` stream event with `is_error: true`)
    pass through as `AgentResult.is_error`.
    """
    cmd = [
        "claude",
        "-p",
        prompt,
        "--model",
        model,
        "--effort",
        effort,
        "--output-format",
        "stream-json",
        "--verbose",
        "--permission-mode",
        "auto",
        "--no-session-persistence",
    ]
    start = time.time()
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd=str(cwd),
        env=_subscription_env(),
        bufsize=1,
    )

    tokens = tool_uses = 0
    is_error = False
    error_message: str | None = None
    stdout_chunks: list[str] = []

    assert proc.stdout is not None
    for raw in proc.stdout:
        stdout_chunks.append(raw)
        try:
            event = json.loads(raw.rstrip("\n"))
        except json.JSONDecodeError:
            continue
        if event.get("type") == "result":
            usage = event.get("usage", {})
            tokens = (
                usage.get("input_tokens", 0) + usage.get("output_tokens", 0)
            )
            tool_uses = event.get("num_turns", 0)
            if event.get("is_error"):
                is_error = True
                error_message = (
                    event.get("result") or "agent reported is_error=true"
                )

    try:
        rc = proc.wait(
            timeout=max(1, timeout_secs - int(time.time() - start))
        )
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()
        rc = -9
        is_error = True
        error_message = f"agent timeout after {timeout_secs}s"

    stderr = proc.stderr.read() if proc.stderr else ""
    if rc != 0 and not is_error:
        is_error = True
        error_message = (stderr or "".join(stdout_chunks))[:200] or f"exit {rc}"

    return AgentResult(
        tokens=tokens,
        tool_uses=tool_uses,
        duration=int(time.time() - start),
        is_error=is_error,
        error_message=error_message,
    )


def _subscription_env() -> dict[str, str]:
    env = os.environ.copy()
    for k in _API_KEY_ENV_VARS:
        env.pop(k, None)
    return env
