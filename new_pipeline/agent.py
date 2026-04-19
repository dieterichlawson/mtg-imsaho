"""Spawn the `claude` CLI in a subprocess and collect a typed result.

Two entry points:

- `run_agent(prompt, cwd, ...)` — one invocation of the `claude` binary.
  Streams stream-json events, returns an `AgentResult` carrying tokens,
  duration, and any error. The child runs with `ANTHROPIC_API_KEY` /
  `ANTHROPIC_AUTH_TOKEN` scrubbed from its environment so it falls back
  to subscription auth. When `sandbox_settings_path` is given, the
  invocation is wrapped in `srt --settings <path> -c '<claude cmd>'`
  for OS-level filesystem and network restrictions. When `log_path` is
  given, each raw stream-json event is appended there (line-buffered)
  AND a short human-readable summary is printed to stdout — so a long
  agent run isn't an opaque wait.
- `run_agent_loop(build_prompt, load_result, ...)` — retry wrapper.
  Calls `run_agent` up to `max_attempts` times; each failing attempt's
  reason is prepended as a `## Retry note` to the next prompt via
  `build_prompt(retry_note, attempt)`. Post-condition: the returned
  `parsed` is None iff `AgentResult.is_error` is True.
"""

from __future__ import annotations

import dataclasses
import json
import os
import shlex
import subprocess
import time
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

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
    sandbox_settings_path: Path | None = None,
    log_path: Path | None = None,
    progress_prefix: str = "",
    extra_env: dict[str, str] | None = None,
    timeout_secs: int = 3600,
) -> AgentResult:
    """Run `claude -p <prompt>` in `cwd`, collect usage + errors, return them.

    The child is killed if it runs past `timeout_secs`, and the returned
    `AgentResult` has `is_error=True` with a timeout message in that case.
    Agent-reported errors (the `result` stream event with `is_error: true`)
    pass through as `AgentResult.is_error`.

    When `sandbox_settings_path` is given, the claude invocation is
    wrapped in `srt --settings <path> -c '<claude cmd>'` so it runs
    under an OS-level filesystem + network sandbox. Without it, claude
    runs directly with no extra restrictions beyond the user's
    standard settings hierarchy.

    When `log_path` is given, every raw stream-json line is appended
    there (line-buffered, full prompt prepended as the first record)
    AND every interesting event yields a one-line summary on stdout
    prefixed with `progress_prefix` so parallel runs stay readable.
    """
    claude_argv = [
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
    if sandbox_settings_path is not None:
        # `srt cmd args...` mangles multi-word args (the prompt comes
        # back to claude as just "Use"). The `-c '<shell-string>'` form
        # works correctly. shlex.join handles the quoting for us.
        cmd = [
            "srt",
            "--settings",
            str(sandbox_settings_path),
            "-c",
            shlex.join(claude_argv),
        ]
    else:
        cmd = claude_argv
    log_fh = None
    if log_path is not None:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_fh = log_path.open("w")
        log_fh.write(json.dumps({"kind": "prompt", "value": prompt}) + "\n")
        log_fh.flush()

    env = _subscription_env()
    if extra_env:
        env.update(extra_env)

    start = time.time()
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd=str(cwd),
        env=env,
        bufsize=1,
    )

    tokens = tool_uses = 0
    is_error = False
    error_message: str | None = None
    stdout_chunks: list[str] = []

    assert proc.stdout is not None
    for raw in proc.stdout:
        stdout_chunks.append(raw)
        if log_fh is not None:
            log_fh.write(raw)
            log_fh.flush()
        try:
            event = json.loads(raw.rstrip("\n"))
        except json.JSONDecodeError:
            continue
        summary = _summarize_stream_event(event)
        if summary:
            print(f"{progress_prefix}{summary}", flush=True)
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

    if log_fh is not None:
        log_fh.close()

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


def _summarize_stream_event(event: dict) -> str | None:
    """Render one stream-json event as a short progress line, or None.

    Returns None when the event isn't worth surfacing (system messages,
    intermediate state). Only `assistant` (tool_use + non-empty text)
    and `result` events produce summaries.
    """
    et = event.get("type")
    if et == "assistant":
        msg = event.get("message", {})
        for block in msg.get("content", []) or []:
            btype = block.get("type")
            if btype == "tool_use":
                name = block.get("name", "tool")
                inp = block.get("input", {}) or {}
                hint = ""
                if name == "Bash":
                    hint = (inp.get("command") or "")[:70]
                elif name in ("Read", "Write", "Edit"):
                    hint = str(inp.get("file_path") or "")[-70:]
                elif name in ("Grep", "Glob"):
                    hint = (inp.get("pattern") or "")[:70]
                return f"[{name}] {hint}".rstrip()
            if btype == "text":
                text = (block.get("text") or "").strip()
                if text:
                    first = text.split("\n", 1)[0]
                    return f"(agent) {first[:120]}"
    elif et == "result":
        if event.get("is_error"):
            return f"(error) {(event.get('result') or '')[:120]}"
        return "(done)"
    return None


def _subscription_env() -> dict[str, str]:
    env = os.environ.copy()
    for k in _API_KEY_ENV_VARS:
        env.pop(k, None)
    return env


# ─── Retry loop ────────────────────────────────────────────────────


BuildPrompt = Callable[[str, int], str]
LoadResult = Callable[["AgentResult", int], tuple[Any, str | None]]


def run_agent_loop(
    *,
    build_prompt: BuildPrompt,
    cwd: Path,
    load_result: LoadResult,
    model: str,
    effort: str,
    sandbox_settings_path: Path | None = None,
    log_dir: Path | None = None,
    log_stem: str = "",
    progress_prefix: str = "",
    extra_env: dict[str, str] | None = None,
    max_attempts: int = 3,
) -> tuple[Any, AgentResult]:
    """Spawn the agent up to `max_attempts` until `load_result` accepts.

    Each attempt:
        1. Build the prompt via `build_prompt(retry_note, attempt)`.
           On attempt 1 the retry_note is empty; on later attempts it
           carries the previous attempt's failure reason.
        2. Spawn the agent via `run_agent`.
        3. Hand the `AgentResult` to `load_result(result, attempt)`. It
           returns `(parsed, None)` on success or `(parsed_or_none,
           reason)` to ask for a retry.

    Returns `(parsed, result)` for the last attempt. Post-condition:
    `parsed is None` iff `result.is_error` is True — after a fully-
    failed run, the returned result is stamped `is_error=True` with
    the last retry reason as `error_message`, even if the agent itself
    technically succeeded but produced unusable output.
    """
    assert max_attempts >= 1, "max_attempts must be at least 1"
    retry_note = ""
    parsed: Any = None
    error: str | None = None
    result: AgentResult | None = None
    for attempt in range(1, max_attempts + 1):
        prompt = build_prompt(retry_note, attempt)
        log_path = (
            log_dir / f"{log_stem}.attempt-{attempt}.jsonl"
            if log_dir is not None and log_stem else None
        )
        result = run_agent(
            prompt,
            cwd=cwd,
            model=model,
            effort=effort,
            sandbox_settings_path=sandbox_settings_path,
            log_path=log_path,
            progress_prefix=progress_prefix,
            extra_env=extra_env,
        )
        parsed, error = load_result(result, attempt)
        if error is None:
            return parsed, result
        if attempt < max_attempts:
            retry_note = (
                f"\n\n## Retry note (attempt {attempt} failed)\n{error}\n"
            )

    # Exhausted attempts with a non-None error. Enforce the invariant
    # that parsed is None iff result.is_error — so callers can branch
    # on result.is_error alone. `parsed` may have come back non-None
    # if the validator rejected an otherwise-valid parse; null it out
    # and stamp the error on the result.
    assert result is not None  # loop ran at least once
    if not result.is_error:
        result = dataclasses.replace(
            result,
            is_error=True,
            error_message=result.error_message or error,
        )
    return None, result


def single_file_loader(
    staging_path: Path,
    parser: Callable[[Path], Any],
    *,
    validator: Callable[[Any], str | None] | None = None,
    missing_hint: str = "",
) -> LoadResult:
    """Build a `load_result` for the common "one file, one parser" case.

    The returned closure, per attempt:
      - surfaces agent errors verbatim,
      - requires `staging_path` to exist on disk (the agent wrote it),
      - parses it with `parser`, raising StagingError → retry note,
      - optionally runs `validator(parsed)`; a non-None return is the
        retry reason (and `parsed` still comes back so callers that
        want the partial can inspect it).

    `missing_hint` gets appended to the "agent did not write <file>"
    message so the retry prompt tells the agent what to do.
    """
    # Deferred import so StagingError isn't required by callers that
    # never trigger the parse-error branch — and to avoid a hard
    # dependency from agent.py on types.py.
    from new_pipeline.types import StagingError

    def _load(result: AgentResult, _attempt: int):
        if result.is_error:
            return None, f"agent error: {result.error_message}"
        if not staging_path.exists():
            suffix = f" {missing_hint}" if missing_hint else ""
            return None, (
                f"agent did not write {staging_path.name}.{suffix}"
            )
        try:
            parsed = parser(staging_path)
        except StagingError as e:
            return None, f"staging JSON failed validation: {e}"
        if validator is not None:
            err = validator(parsed)
            if err is not None:
                return parsed, err
        return parsed, None
    return _load
