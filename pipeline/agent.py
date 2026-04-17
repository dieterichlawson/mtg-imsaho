"""Agent subprocess runner, retry loop, and prompt assembly."""
from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path
from typing import Any, Callable

from pipeline._staging import StagingError

DEFAULT_MODEL = "opus"
DEFAULT_EFFORT = "max"
AGENT_TIMEOUT_SECS = 3600
MAX_ATTEMPTS = 3
API_KEY_ENV_VARS = ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN")


def subscription_env() -> dict:
    env = os.environ.copy()
    for k in API_KEY_ENV_VARS:
        env.pop(k, None)
    return env


def _summarize_stream_event(event: dict) -> str | None:
    """Short progress line for a stream-json event, or None to skip."""
    et = event.get("type")
    if et == "result":
        if event.get("is_error"):
            return f"(error) {(event.get('result') or '')[:120]}"
        return "(done)"
    if et != "assistant":
        return None
    for block in event.get("message", {}).get("content") or []:
        bt = block.get("type")
        if bt == "tool_use":
            name = block.get("name", "tool")
            inp = block.get("input", {}) or {}
            hint = (inp.get("command")  if name == "Bash"
                else inp.get("file_path") if name in ("Read", "Write", "Edit")
                else inp.get("pattern")   if name in ("Grep", "Glob")
                else "") or ""
            return f"[{name}] {str(hint)[:70]}".rstrip()
        if bt == "text":
            text = (block.get("text") or "").strip()
            if text:
                return f"(agent) {text.split(chr(10), 1)[0][:120]}"
    return None


def run_agent_in(prompt: str, cwd: Path,
                 model: str = DEFAULT_MODEL, effort: str = DEFAULT_EFFORT,
                 log_path: Path | None = None,
                 progress_prefix: str = "",
                 settings_path: Path | None = None) -> dict:
    """Run claude in `cwd`, streaming stream-json to stdout + log_path."""
    cmd = ["claude", "-p", prompt, "--model", model, "--effort", effort,
           "--output-format", "stream-json", "--verbose",
           "--permission-mode", "auto", "--no-session-persistence"]
    if settings_path and settings_path.exists():
        cmd += ["--settings", str(settings_path)]

    log_fh = None
    if log_path is not None:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_fh = log_path.open("w")
        log_fh.write(json.dumps({"kind": "prompt", "value": prompt}) + "\n")
        log_fh.flush()

    start = time.time()
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            text=True, cwd=str(cwd), env=subscription_env(),
                            bufsize=1)
    tokens = tool_uses = 0
    is_error = False
    error_message: str | None = None
    stdout_chunks: list[str] = []
    try:
        assert proc.stdout is not None
        for raw in proc.stdout:
            stdout_chunks.append(raw)
            if log_fh is not None:
                log_fh.write(raw); log_fh.flush()
            try:
                event = json.loads(raw.rstrip("\n"))
            except json.JSONDecodeError:
                continue
            summary = _summarize_stream_event(event)
            if summary:
                print(f"{progress_prefix}{summary}", flush=True)
            if event.get("type") == "result":
                u = event.get("usage", {})
                tokens = u.get("input_tokens", 0) + u.get("output_tokens", 0)
                tool_uses = event.get("num_turns", 0)
                if event.get("is_error"):
                    is_error = True
                    error_message = event.get("result") or "agent reported is_error=true"
        try:
            rc = proc.wait(timeout=max(1, AGENT_TIMEOUT_SECS - int(time.time() - start)))
        except subprocess.TimeoutExpired:
            proc.kill(); proc.wait()
            rc = -9
            is_error = True
            error_message = f"agent timeout after {AGENT_TIMEOUT_SECS}s"
    finally:
        if log_fh is not None:
            log_fh.close()
    stderr = proc.stderr.read() if proc.stderr else ""
    if rc != 0 and not is_error:
        is_error = True
        error_message = (stderr or "".join(stdout_chunks))[:200] or f"exit {rc}"
    return {"returncode": rc, "tokens": tokens, "tool_uses": tool_uses,
            "duration": int(time.time() - start), "is_error": is_error,
            "error_message": error_message}


def run_agent_loop(
    *, build_prompt: Callable[[str, int], str], cwd: Path,
    staging_file: Path, loader: Callable[[Path], Any],
    validator: Callable[[Any, dict, int], str | None] | None = None,
    max_attempts: int = MAX_ATTEMPTS,
    model: str = DEFAULT_MODEL, effort: str = DEFAULT_EFFORT,
    logs_dir: Path, log_prefix: str = "", progress_prefix: str = "",
    settings_path: Path | None = None,
    spawn: Callable[..., dict] | None = None,
) -> tuple[Any, dict]:
    """Spawn the agent until it produces valid staging + passes validator,
    up to max_attempts. Failure reason becomes a `## Retry note` on the next
    prompt. Returns (parsed, last_result); parsed is None iff every attempt
    failed before a valid load.

    `spawn` lets callers pass a custom subprocess runner (primarily for tests
    that patch cli.run_agent_in); defaults to the module-level run_agent_in."""
    _spawn = spawn or run_agent_in
    retry_note = ""
    parsed: Any = None
    result: dict = {"duration": 0, "tokens": 0, "tool_uses": 0,
                    "is_error": False, "error_message": None}
    for attempt in range(1, max_attempts + 1):
        prompt = build_prompt(retry_note, attempt)
        log_path = logs_dir / f"{log_prefix}-attempt{attempt}.log"
        result = _spawn(prompt, cwd, model, effort,
                        log_path=log_path,
                        progress_prefix=progress_prefix,
                        settings_path=settings_path)
        err: str | None = None
        parsed = None
        if result.get("is_error"):
            err = f"Previous attempt errored: {result.get('error_message')}"
        elif not staging_file.exists():
            err = (f"Previous attempt did not write {staging_file.name}. "
                   f"Write your staging output there.")
        else:
            try:
                parsed = loader(staging_file)
            except StagingError as e:
                err = (f"Your staging JSON failed validation: {e}\n"
                       f"Re-emit matching the shared prompt's schema.")
            # Staging is ephemeral; subsequent command-level checks assume
            # it's gone.
            if staging_file.exists():
                staging_file.unlink()
        if err is None and validator is not None:
            err = validator(parsed, result, attempt)
        if err is None:
            return parsed, result
        print(f"{progress_prefix}attempt {attempt} rejected: "
              f"{err.splitlines()[0]}")
        if attempt == max_attempts:
            return parsed, result
        retry_note = f"\n\n## Retry note (attempt {attempt} failed)\n{err}\n"
    return parsed, result


# Map role → per-agent template basename.
_PER_AGENT_NAME = {"auditor": "audit", "test-writer": "test",
                   "fixer": "fix", "dedup": "dedup"}


class _SafeDict(dict):
    def __missing__(self, key: str) -> str:
        return "{" + key + "}"


def build_prompt(role: str, prompts_dir: Path,
                 **ctx) -> Callable[[str, int], str]:
    """Return a `(retry_note, attempt) -> full prompt` builder.
    Concatenates the shared role prompt with the filled per-agent template."""
    shared = (prompts_dir / f"{role}.md").read_text()
    per_path = prompts_dir / f"{_PER_AGENT_NAME[role]}.peragent.md"
    template = per_path.read_text() if per_path.exists() else ""
    per_agent = template.format_map(_SafeDict(ctx)) if template else ""
    def builder(retry_note: str, _attempt: int) -> str:
        return shared + "\n\n---\n\n" + per_agent + retry_note
    return builder
