"""Agent subprocess runner, streaming log, retry loop, and prompt assembly.

Every Claude invocation in the pipeline goes through `run_agent_loop`,
which spawns `claude` via `run_agent_in`, parses stream-json events,
loads the agent's staging JSON output, and retries up to MAX_ATTEMPTS
times on failure.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from collections.abc import Callable
from pathlib import Path
from typing import Any

from pipeline import utils
from pipeline.models import StagingError

DEFAULT_MODEL = "opus"
DEFAULT_EFFORT = "max"
AGENT_TIMEOUT_SECS = 3600
MAX_ATTEMPTS = 3

# Env vars that force API-key billing when set. Scrubbed from agent
# subprocesses so `claude` falls back to subscription auth.
API_KEY_ENV_VARS = ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN")


def _subscription_env() -> dict:

    env = os.environ.copy()
    for k in API_KEY_ENV_VARS:
        env.pop(k, None)
    return env


# ─── Streaming subprocess ──────────────────────────────────────────


def run_agent_in(
    prompt: str,
    cwd: Path,
    model: str = DEFAULT_MODEL,
    effort: str = DEFAULT_EFFORT,
    log_path: Path | None = None,
    progress_prefix: str = "",
) -> dict:
    """Run `claude` in `cwd`, streaming stream-json events to stdout and.

    (if given) `log_path`. Returns a result dict with tokens, tool_uses,
    duration, is_error, error_message.
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
    if utils.AGENT_SETTINGS.exists():
        cmd += ["--settings", str(utils.AGENT_SETTINGS)]

    log_fh = None
    if log_path is not None:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_fh = log_path.open("w")
        log_fh.write(json.dumps({"kind": "prompt", "value": prompt}) + "\n")
        log_fh.flush()

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
    try:
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
            summary = _stream_summary(event)
            if summary:
                print(f"{progress_prefix}{summary}", flush=True)
            if event.get("type") == "result":
                u = event.get("usage", {})
                tokens = u.get("input_tokens", 0) + u.get("output_tokens", 0)
                tool_uses = event.get("num_turns", 0)
                if event.get("is_error"):
                    is_error = True
                    error_message = (
                        event.get("result") or "agent reported is_error=true"
                    )
        try:
            rc = proc.wait(
                timeout=max(1, AGENT_TIMEOUT_SECS - int(time.time() - start))
            )
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
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
    return {
        "returncode": rc,
        "tokens": tokens,
        "tool_uses": tool_uses,
        "duration": int(time.time() - start),
        "is_error": is_error,
        "error_message": error_message,
    }


def worktree_staging_file(
    worktree: Path, ticket_id: str, stage: str,
) -> Path:
    """Path the agent should write its staging JSON to inside a worktree.

    `stage` is "test", "fix", etc. — it tags the filename so concurrent
    stages on one worktree can't clobber each other.
    """
    staging = worktree / "pipeline" / "staging"
    staging.mkdir(parents=True, exist_ok=True)
    return staging / f"{ticket_id}-{stage}.json"


def run_agent(
    prompt: str,
    model: str = DEFAULT_MODEL,
    effort: str = DEFAULT_EFFORT,
    log_path: Path | None = None,
    progress_prefix: str = "",
) -> dict:
    """Shorthand for running at PROJECT_ROOT (no worktree)."""
    return run_agent_in(
        prompt,
        utils.PROJECT_ROOT,
        model,
        effort,
        log_path=log_path,
        progress_prefix=progress_prefix,
    )


def _stream_summary(event: dict) -> str | None:
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
            hint = (
                inp.get("command")
                if name == "Bash"
                else inp.get("file_path")
                if name in ("Read", "Write", "Edit")
                else inp.get("pattern")
                if name in ("Grep", "Glob")
                else ""
            ) or ""
            return f"[{name}] {str(hint)[:70]}".rstrip()
        if bt == "text":
            text = (block.get("text") or "").strip()
            if text:
                return f"(agent) {text.split(chr(10), 1)[0][:120]}"
    return None


# ─── Retry loop ─────────────────────────────────────────────────────


LoadResult = Callable[[dict, int], tuple[Any, str | None]]


def run_agent_loop(
    *,
    build_prompt: Callable[[str, int], str],
    cwd: Path,
    load_result: LoadResult,
    max_attempts: int = MAX_ATTEMPTS,
    model: str = DEFAULT_MODEL,
    effort: str = DEFAULT_EFFORT,
    log_prefix: str = "",
    progress_prefix: str = "",
) -> tuple[Any, dict]:
    """Spawn the agent until `load_result` accepts its output, up to.

    max_attempts. Every failure's reason becomes a `## Retry note`
    appended to the next prompt.

    `load_result(result, attempt)` is the caller's hook: it inspects
    the raw agent result (+ any worktree/staging side effects) and
    returns `(parsed, None)` on success or `(None, reason)` on failure.

    Post-condition: `parsed is None` iff `result["is_error"]` is True —
    callers can rely on a non-None `parsed` whenever `is_error` is False.
    """
    retry_note = ""
    parsed: Any = None
    error: str | None = "no attempts ran"
    result: dict = {}
    for attempt in range(1, max_attempts + 1):
        prompt = build_prompt(retry_note, attempt)
        log_path = utils.LOGS_DIR / f"{log_prefix}-attempt{attempt}.log"
        result = run_agent_in(
            prompt, cwd, model, effort,
            log_path=log_path, progress_prefix=progress_prefix,
        )
        parsed, error = load_result(result, attempt)
        if error is None:
            return parsed, result
        print(
            f"{progress_prefix}attempt {attempt} rejected: "
            f"{error.splitlines()[0]}"
        )
        if attempt < max_attempts:
            retry_note = (
                f"\n\n## Retry note (attempt {attempt} failed)\n{error}\n"
            )

    # Exhausted attempts. Stamp is_error so callers can rely on the invariant.
    if parsed is None:
        result["is_error"] = True
        if not result.get("error_message"):
            result["error_message"] = error
    return parsed, result


def single_file_loader(
    staging_file: Path,
    loader: Callable[[Path], Any],
    validator: Callable[[Any, dict, int], str | None] | None = None,
) -> LoadResult:
    """Load a single staging file and optionally run a follow-up validator.

    The staging file is always deleted after loading (ephemeral transport;
    worktree-clean checks assume it's gone). On StagingError the file is
    still deleted before returning the retry note.
    """
    def _load(result: dict, attempt: int) -> tuple[Any, str | None]:
        if result.get("is_error"):
            err_msg = result.get("error_message") or "unknown"
            return None, f"Previous attempt errored: {err_msg}"
        if not staging_file.exists():
            return None, (
                f"Previous attempt did not write {staging_file.name}. "
                f"Write your staging output there."
            )
        try:
            parsed = loader(staging_file)
        except StagingError as e:
            staging_file.unlink()
            return None, (
                f"Your staging JSON failed validation: {e}\n"
                f"Re-emit matching the shared prompt's schema."
            )
        staging_file.unlink()
        if validator is not None:
            err = validator(parsed, result, attempt)
            if err is not None:
                return parsed, err
        return parsed, None
    return _load


# ─── Prompt assembly ────────────────────────────────────────────────

_PER_AGENT_NAME = {
    "auditor": "audit",
    "test-writer": "test",
    "fixer": "fix",
    "dedup": "dedup",
}


class _SafeDict(dict):
    """format_map() helper: returns `{key}` for missing keys so a template.

    tolerates more placeholders than the caller fills.
    """

    def __missing__(self, key: str) -> str:
        return "{" + key + "}"


def build_prompt(role: str, **ctx) -> Callable[[str, int], str]:
    """Return `(retry_note, attempt) → full prompt`. Concatenates the role's.

    shared prompt (`prompts/<role>.md`) with the filled per-agent template
    (`prompts/<short>.peragent.md`) and any retry note.
    """
    shared = (utils.PROMPTS_DIR / f"{role}.md").read_text()
    per_path = utils.PROMPTS_DIR / f"{_PER_AGENT_NAME[role]}.peragent.md"
    per_template = per_path.read_text() if per_path.exists() else ""
    per_agent = per_template.format_map(_SafeDict(ctx)) if per_template else ""

    def builder(retry_note: str, _attempt: int) -> str:
        return shared + "\n\n---\n\n" + per_agent + retry_note

    return builder
