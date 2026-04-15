#!/usr/bin/env python3
"""Pipeline CLI — manage bug-finding and fixing agents.

Tickets are the atomic unit. Each ticket tracks one bug through:
  new → confirmed → fixed → merged
     ↘ rejected (terminal)
     ↘ blocked (manual intervention needed)
                ↘ failed (can retry)

Agents write to staging/. Python owns ticket state and frontmatter.

Usage:
    ./pipeline/cli.py audit --cards "Olivia Voldaren,Fiend Hunter"
    ./pipeline/cli.py test
    ./pipeline/cli.py fix --ticket olivia-01
    ./pipeline/cli.py tickets --status new
    ./pipeline/cli.py show olivia-01
    ./pipeline/cli.py accept olivia-01
    ./pipeline/cli.py status
"""

import argparse
import concurrent.futures
import json
import os
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# ─── Paths ────────────────────────────────────────────────────────

PROJECT_ROOT = Path(__file__).resolve().parent.parent
PIPELINE_DIR = PROJECT_ROOT / "pipeline"
TICKETS_DIR = PIPELINE_DIR / "tickets"
STAGING_DIR = PIPELINE_DIR / "staging"
PROMPTS_DIR = PIPELINE_DIR / "prompts"
SCRIPTS_DIR = PIPELINE_DIR / "scripts"
METRICS_DIR = PIPELINE_DIR / "metrics"
LOGS_DIR = PIPELINE_DIR / "logs"
CARDS_DIR = PROJECT_ROOT / "mtg-engine" / "src" / "cards" / "isd"
ORACLE_SCRIPT = PROJECT_ROOT / "scripts" / "oracle_lookup.py"

DEFAULT_MODEL = "opus"

# Max wall-clock an agent subprocess may run before SIGKILL. Large dedup
# passes with many tickets to cross-reference exceed the old 15-minute
# limit; bumped to 1 hour.
AGENT_TIMEOUT_SECS = 3600

# Env vars that force API-key billing when set. Scrubbed from agent
# subprocesses so the `claude` CLI falls back to Claude Code subscription auth.
API_KEY_ENV_VARS = ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN")


def subscription_env() -> dict:
    """Inherit current env but strip API-key vars so subprocesses use
    the Claude Code subscription rather than pay-as-you-go API billing."""
    env = os.environ.copy()
    for k in API_KEY_ENV_VARS:
        env.pop(k, None)
    return env
DEFAULT_EFFORT = "max"


# ─── Utilities ────────────────────────────────────────────────────

def now_iso():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

def today():
    return datetime.now().strftime("%Y-%m-%d")

def card_to_snake(name: str) -> str:
    return name.lower().replace(" ", "_").replace("'", "").replace(",", "").replace("-", "_")

def append_jsonl(path: Path, entry: dict):
    with open(path, "a") as f:
        f.write(json.dumps(entry) + "\n")

def _text_similarity(a: str, b: str) -> float:
    """Simple word-overlap similarity between two strings."""
    words_a = set(a.split())
    words_b = set(b.split())
    if not words_a or not words_b:
        return 0.0
    overlap = len(words_a & words_b)
    return overlap / max(len(words_a), len(words_b))


WORKTREES_DIR = PROJECT_ROOT / ".worktrees"


def get_worktree_dir(ticket_id: str) -> Path:
    return WORKTREES_DIR / f"fix-{ticket_id}"


def get_worktree_branch(ticket_id: str) -> str:
    return f"fix/{ticket_id}"


def ensure_worktree(ticket_id: str) -> Path:
    """Create or reuse a worktree for a ticket. Returns the worktree path."""
    wt_dir = get_worktree_dir(ticket_id)
    branch = get_worktree_branch(ticket_id)
    WORKTREES_DIR.mkdir(parents=True, exist_ok=True)

    if wt_dir.exists():
        # Worktree already exists (reuse for fix phase after test phase)
        return wt_dir

    # Create fresh worktree from HEAD
    subprocess.run(
        ["git", "worktree", "add", "-b", branch, str(wt_dir), "HEAD"],
        capture_output=True, check=True, cwd=str(PROJECT_ROOT),
    )
    return wt_dir


def remove_worktree(ticket_id: str):
    """Remove a ticket's worktree and branch."""
    wt_dir = get_worktree_dir(ticket_id)
    branch = get_worktree_branch(ticket_id)
    if wt_dir.exists():
        subprocess.run(["git", "worktree", "remove", "--force", str(wt_dir)],
                      capture_output=True, cwd=str(PROJECT_ROOT))
    subprocess.run(["git", "branch", "-D", branch],
                  capture_output=True, cwd=str(PROJECT_ROOT))


def remove_logs_for_ticket(ticket_id: str) -> int:
    """Delete any agent-output log files whose filename mentions
    `ticket_id`. Run at merge/abandon time — the ticket's audit/test/
    fix history is no longer actionable and the logs were only kept
    for replay/post-mortem debugging.

    Returns the count of files removed."""
    if not LOGS_DIR.exists():
        return 0
    removed = 0
    for f in LOGS_DIR.glob(f"*{ticket_id}*"):
        try:
            f.unlink()
            removed += 1
        except OSError:
            pass
    return removed


def merge_worktree(ticket_id: str) -> bool:
    """Merge a ticket's worktree branch into HEAD. Returns success."""
    branch = get_worktree_branch(ticket_id)
    result = subprocess.run(
        ["git", "merge", branch, "--no-edit"],
        capture_output=True, text=True, cwd=str(PROJECT_ROOT),
    )
    return result.returncode == 0


def _summarize_stream_event(event: dict) -> str | None:
    """Turn one stream-json event into a short human-readable line for
    real-time progress. Returns None if the event isn't interesting.
    Keep lines short so parallel-agent prefixes stay readable."""
    et = event.get("type")
    if et == "assistant":
        msg = event.get("message", {})
        for block in msg.get("content", []) or []:
            btype = block.get("type")
            if btype == "tool_use":
                name = block.get("name", "tool")
                inp = block.get("input", {}) or {}
                # Pull out a short identifier depending on the tool
                hint = ""
                if name == "Bash":
                    hint = (inp.get("command") or "")[:70]
                elif name in ("Read", "Write"):
                    hint = str(inp.get("file_path") or "")[-70:]
                elif name == "Edit":
                    hint = str(inp.get("file_path") or "")[-70:]
                elif name == "Grep":
                    hint = (inp.get("pattern") or "")[:70]
                elif name == "Glob":
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


def run_agent_in(prompt: str, cwd: Path, model: str = DEFAULT_MODEL,
                 effort: str = DEFAULT_EFFORT,
                 log_path: Path | None = None,
                 progress_prefix: str = "") -> dict:
    """Run a claude agent in a specific directory. Streams stream-json
    events from the agent as they arrive: each event is appended to
    `log_path` immediately and a short human-readable summary is
    printed to stdout (prefixed with `progress_prefix` so parallel
    runs are distinguishable). Returns usage stats accumulated from
    the final `result` event."""
    cmd = [
        "claude", "-p", prompt,
        "--model", model,
        "--effort", effort,
        "--output-format", "stream-json",
        "--verbose",
        "--permission-mode", "auto",
        "--no-session-persistence",
    ]

    # Open log file up front so every event hits disk as it arrives.
    log_fh = None
    if log_path is not None:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_fh = log_path.open("w")
        log_fh.write(json.dumps({"kind": "prompt", "value": prompt}) + "\n")
        log_fh.flush()

    start = time.time()
    proc = subprocess.Popen(
        cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, cwd=str(cwd), env=subscription_env(),
        bufsize=1,  # line-buffered
    )

    tokens = 0
    tool_uses = 0
    is_error = False
    error_message = None
    final_event: dict | None = None
    stdout_chunks: list[str] = []

    try:
        assert proc.stdout is not None
        for raw_line in proc.stdout:
            line = raw_line.rstrip("\n")
            stdout_chunks.append(raw_line)
            if log_fh is not None:
                log_fh.write(raw_line)
                log_fh.flush()
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            summary = _summarize_stream_event(event)
            if summary:
                print(f"{progress_prefix}{summary}", flush=True)
            if event.get("type") == "result":
                final_event = event
                tokens = event.get("usage", {}).get("input_tokens", 0) + \
                         event.get("usage", {}).get("output_tokens", 0)
                tool_uses = event.get("num_turns", 0)
                if event.get("is_error"):
                    is_error = True
                    error_message = event.get("result") or "agent reported is_error=true"
        # Enforce the wall-clock ceiling ourselves since we don't pass
        # timeout= to Popen.
        try:
            rc = proc.wait(timeout=max(1, AGENT_TIMEOUT_SECS - int(time.time() - start)))
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
    elapsed = int(time.time() - start)

    if rc != 0 and not is_error:
        is_error = True
        error_message = (stderr or "".join(stdout_chunks))[:200] or f"exit {rc}"

    return {"returncode": rc, "tokens": tokens, "tool_uses": tool_uses,
            "duration": elapsed, "is_error": is_error,
            "error_message": error_message}


def get_oracle_text(card_name: str) -> str | None:
    for cmd in ["lookup", "fetch"]:
        r = subprocess.run(
            ["python3", str(ORACLE_SCRIPT), cmd, card_name],
            capture_output=True, text=True, cwd=str(PROJECT_ROOT),
        )
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
    return None


# ─── Ticket I/O ───────────────────────────────────────────────────

def parse_ticket(path: Path) -> dict:
    """Parse a ticket file into {frontmatter: dict, body: str}."""
    text = path.read_text()
    if not text.startswith("---"):
        return {"frontmatter": {}, "body": text, "path": path}
    try:
        end = text.index("---", 3)
    except ValueError:
        return {"frontmatter": {}, "body": text, "path": path}
    fm = {}
    for line in text[3:end].strip().split("\n"):
        if ":" in line:
            key, val = line.split(":", 1)
            fm[key.strip()] = val.strip()
    body = text[end+3:].strip()
    return {"frontmatter": fm, "body": body, "path": path}


def write_ticket(ticket_id: str, frontmatter: dict, body: str):
    """Write a ticket file. Python owns this — agents never call it."""
    path = TICKETS_DIR / f"{ticket_id}.md"
    fm_lines = ["---"]
    for k, v in frontmatter.items():
        fm_lines.append(f"{k}: {v}")
    fm_lines.append("---")
    content = "\n".join(fm_lines) + "\n\n" + body + "\n"
    path.write_text(content)
    return path


def update_ticket_status(ticket_id: str, new_status: str, extra_fm: dict = None):
    """Update a ticket's status and optionally add frontmatter fields."""
    path = TICKETS_DIR / f"{ticket_id}.md"
    ticket = parse_ticket(path)
    fm = ticket["frontmatter"]
    fm["status"] = new_status
    if extra_fm:
        fm.update(extra_fm)
    write_ticket(ticket_id, fm, ticket["body"])


def append_ticket_section(ticket_id: str, section: str):
    """Append a section to a ticket's body."""
    path = TICKETS_DIR / f"{ticket_id}.md"
    ticket = parse_ticket(path)
    new_body = ticket["body"] + "\n\n" + section
    write_ticket(ticket_id, ticket["frontmatter"], new_body)


def list_tickets(status: str = None, card: str = None) -> list[dict]:
    """List all tickets, optionally filtered."""
    tickets = []
    for f in sorted(TICKETS_DIR.glob("*.md")):
        t = parse_ticket(f)
        fm = t["frontmatter"]
        if status and fm.get("status") != status:
            continue
        if card and card.lower() not in fm.get("card", "").lower():
            continue
        tickets.append(t)
    return tickets


# ─── Agent runner ─────────────────────────────────────────────────

def run_agent(prompt: str, model: str = DEFAULT_MODEL,
              effort: str = DEFAULT_EFFORT,
              log_path: Path | None = None,
              progress_prefix: str = "") -> dict:
    """Run a claude agent at PROJECT_ROOT. Thin wrapper around
    run_agent_in for callers that don't need a worktree."""
    return run_agent_in(prompt, PROJECT_ROOT, model, effort,
                        log_path=log_path, progress_prefix=progress_prefix)


# ─── Audit staging parser ────────────────────────────────────────

def parse_audit_staging(staging_path: Path) -> dict:
    """Parse structured audit output from staging into findings + metadata."""
    text = staging_path.read_text()

    # Extract checks performed
    checks = {}
    checks_match = re.search(r"## Checks Performed\n(.*?)(?=\n## )", text, re.DOTALL)
    if checks_match:
        for line in checks_match.group(1).strip().split("\n"):
            line = line.strip()
            if line and ":" in line:
                check_id, rest = line.split(":", 1)
                checks[check_id.strip()] = rest.strip()

    # Extract findings
    findings = []
    finding_blocks = re.split(r"\n## Finding \d+", text)
    for block in finding_blocks[1:]:  # skip everything before first finding
        finding = {}

        oracle_match = re.search(r"\*\*Oracle text:\*\*\n>(.+?)(?=\n\*\*)", block, re.DOTALL)
        if oracle_match:
            finding["oracle_quote"] = oracle_match.group(1).strip()

        code_match = re.search(r"\*\*Code:\*\*\n>(.+?)(?=\n\*\*|\n##)", block, re.DOTALL)
        if code_match:
            finding["code_quote"] = code_match.group(1).strip()

        desc_match = re.search(r"\*\*Description:\*\*\n(.+?)(?=\n\*\*|\n##)", block, re.DOTALL)
        if desc_match:
            finding["description"] = desc_match.group(1).strip()

        path_match = re.search(r"\*\*Engine path:\*\*\n(.+?)(?=\n\*\*|\n##)", block, re.DOTALL)
        if path_match:
            finding["engine_path"] = path_match.group(1).strip()

        check_match = re.search(r"\*\*Check:\*\*\s*(.+)", block)
        if check_match:
            finding["check"] = check_match.group(1).strip()

        affected_match = re.search(r"\*\*Affected cards:\*\*\n(.+?)(?=\n\*\*|\n##|$)", block, re.DOTALL)
        if affected_match:
            finding["affected_cards"] = affected_match.group(1).strip()

        # Parse **Tests:** block — zero or more ### {slug} / Scenario: entries
        tests = []
        tests_match = re.search(r"\*\*Tests:\*\*\n(.+?)(?=\n##\s|\Z)", block, re.DOTALL)
        if tests_match:
            tests_body = tests_match.group(1).strip()
            for entry in re.split(r"\n(?=###\s+)", tests_body):
                entry = entry.strip()
                if not entry.startswith("###"):
                    continue
                head, _, rest = entry.partition("\n")
                slug = head[3:].strip()
                scenario_match = re.search(r"Scenario:\s*(.+)", rest, re.DOTALL)
                if slug and scenario_match:
                    tests.append({
                        "slug": slug,
                        "scenario": scenario_match.group(1).strip(),
                    })
        finding["tests"] = tests

        if finding.get("description"):
            findings.append(finding)

    # Extract insights
    insights = []
    insights_match = re.search(r"## Insights\n(.*?)$", text, re.DOTALL)
    if insights_match:
        insights_text = insights_match.group(1).strip()
        if insights_text:
            insights.append(insights_text)

    # Extract untested rulings
    rulings = []
    rulings_match = re.search(r"## Untested Rulings\n(.*?)(?=\n## |\Z)", text, re.DOTALL)
    if rulings_match:
        rulings = [l.strip() for l in rulings_match.group(1).strip().split("\n") if l.strip()]

    # Check for pass
    is_pass = len(findings) == 0

    return {
        "checks": checks,
        "findings": findings,
        "insights": insights,
        "untested_rulings": rulings,
        "is_pass": is_pass,
    }


def parse_test_staging(staging_path: Path) -> dict:
    """Parse test-writer output. Multi-test format produced by the
    test-writer prompt:

        # Test Result: {ticket_id}

        ## Test File
        {shared path}

        ## Test: {slug}
        Status: confirmed | rejected | blocked
        Test name: {fn}
        Assertion message: {...}
        Explanation: {...}
        Blocked by: {only if blocked}

        ## Test: {another_slug}
        ...

    Returns: {test_file, tests: [{slug, status, test_name,
    assertion_message, explanation, blocked_by}]}
    """
    text = staging_path.read_text()
    result = {"test_file": "", "tests": []}

    tf = re.search(r"##\s+Test File\n(.+?)(?=\n##\s|\Z)", text, re.DOTALL)
    if tf:
        result["test_file"] = tf.group(1).strip()

    for block in re.split(r"\n(?=##\s+Test:\s)", text):
        block = block.strip()
        if not block.startswith("## Test:"):
            continue
        head, _, body = block.partition("\n")
        slug = head.replace("## Test:", "").strip()

        def field(name: str, body=body) -> str | None:
            m = re.search(rf"(?m)^{re.escape(name)}:\s*(.*?)(?=\n[A-Z][\w ]*:|\Z)",
                          body, re.DOTALL)
            return m.group(1).strip() if m else None

        result["tests"].append({
            "slug": slug,
            "status": (field("Status") or "rejected").lower().split()[0],
            "test_name": field("Test name") or slug,
            "assertion_message": field("Assertion message") or "",
            "explanation": field("Explanation") or "",
            "blocked_by": field("Blocked by"),
        })

    return result


def update_tests_section_impls(ticket_id: str,
                               impls_by_slug: dict[str, str]) -> None:
    """Rewrite the `Implementation:` line for each matching `### slug` in
    the ticket's `## Tests` section. `impls_by_slug` maps a test slug to
    the string the Implementation field should read after update (e.g.
    'mtg-engine/tests/pipeline_bugs_foo.rs::test_foo' or 'rejected: ...').
    """
    path = TICKETS_DIR / f"{ticket_id}.md"
    t = parse_ticket(path)
    body = t["body"]
    new_lines: list[str] = []
    current_slug: str | None = None
    for line in body.split("\n"):
        if line.startswith("### "):
            current_slug = line[4:].strip()
            new_lines.append(line)
        elif (current_slug and current_slug in impls_by_slug
              and line.startswith("Implementation:")):
            new_lines.append(f"Implementation: {impls_by_slug[current_slug]}")
        else:
            new_lines.append(line)
    write_ticket(ticket_id, t["frontmatter"], "\n".join(new_lines).rstrip() + "\n")


def parse_fix_staging(staging_path: Path) -> dict:
    """Parse fixer output from staging."""
    text = staging_path.read_text()

    result = {}
    for field in ["Status", "Files Changed", "Description"]:
        match = re.search(rf"## {field}\n(.+?)(?=\n## |\Z)", text, re.DOTALL)
        if match:
            result[field.lower().replace(" ", "_")] = match.group(1).strip()

    return result


# ─── Audit command ────────────────────────────────────────────────

def cmd_audit(args):
    sep = ";" if ";" in args.cards else ","
    cards = [c.strip() for c in args.cards.split(sep) if c.strip()]

    print(f"\n{'='*60}")
    print(f"AUDIT — {len(cards)} card(s)")
    print(f"{'='*60}")
    for c in cards:
        print(f"  {c}")

    if args.dry_run:
        print("\n(dry run)")
        return

    # Pre-fetch oracle texts
    print("\nFetching oracle texts...")
    card_oracles = {}
    for card in cards:
        oracle = get_oracle_text(card)
        if oracle:
            card_oracles[card] = oracle
        else:
            print(f"  SKIP: no oracle text for {card}")

    if not card_oracles:
        print("No cards to audit.")
        return

    shared_prompt = (PROMPTS_DIR / "auditor.md").read_text()

    def audit_one(card: str) -> dict:
        card_snake = card_to_snake(card)
        oracle = card_oracles[card]
        run_id = f"{today()}-{card_snake}-audit"
        staging_file = STAGING_DIR / f"{run_id}.md"

        per_agent = f"""## Card to audit: {card}

### Implementation file
`mtg-engine/src/cards/isd/{card_snake}.rs`

### Oracle text (pre-fetched from Scryfall)

{oracle}

### Output
Write your structured audit output to `pipeline/staging/{run_id}.md`.
Use the format specified in the prompt (Checks Performed, Finding N sections, Insights).
"""
        print(f"  [{card}] Spawning agent...")
        log_path = LOGS_DIR / f"{run_id}.log"
        result = run_agent(shared_prompt + "\n\n---\n\n" + per_agent,
                          args.model, args.effort,
                          log_path=log_path,
                          progress_prefix=f"  [{card}] ")

        if result.get("is_error"):
            err = result.get("error_message") or "unknown error"
            print(f"  [{card}] AGENT ERROR: {err} ({result['duration']}s, {result['tokens']} tok)")
            append_jsonl(METRICS_DIR / "runs.jsonl", {
                "run_id": run_id, "timestamp": now_iso(), "role": "auditor",
                "model": args.model, "card": card, "finding_id": None,
                "findings_created": 0,
                "test_result": None, "fix_result": None,
                "validation_passed": False, "rejection_reason": f"agent error: {err}",
                "total_tokens": result["tokens"], "tool_uses": result["tool_uses"],
                "duration_seconds": result["duration"], "notes": "agent_error",
            })
            return {"card": card, "tickets": 0, "duration": result["duration"],
                    "tokens": result["tokens"], "error": err}

        # Parse staging output
        tickets_created = []
        if staging_file.exists():
            parsed = parse_audit_staging(staging_file)

            # Find next available ticket number for this card
            existing = sorted(TICKETS_DIR.glob(f"{card_snake}-*.md"))
            existing_nums = []
            for ef in existing:
                m = re.search(rf"{card_snake}-(\d+)", ef.stem)
                if m:
                    existing_nums.append(int(m.group(1)))
            next_num = max(existing_nums, default=0) + 1

            # Create tickets from findings
            for finding in parsed["findings"]:
                ticket_id = f"{card_snake}-{next_num:02d}"
                next_num += 1
                desc_short = finding.get("description", "")[:80]

                body = "## Audit Finding\n\n"
                if finding.get("oracle_quote"):
                    body += f"**Oracle text:**\n> {finding['oracle_quote']}\n\n"
                if finding.get("code_quote"):
                    body += f"**Code:**\n> {finding['code_quote']}\n\n"
                if finding.get("description"):
                    body += f"**Description:**\n{finding['description']}\n\n"
                if finding.get("engine_path"):
                    body += f"**Engine path:**\n{finding['engine_path']}\n\n"
                if finding.get("check"):
                    body += f"**Required check:** {finding['check']}\n\n"
                if finding.get("affected_cards"):
                    body += f"**Affected cards:**\n{finding['affected_cards']}\n\n"

                # Render ## Tests section. If the auditor supplied tests,
                # emit them verbatim; otherwise scaffold a single default
                # entry so every ticket has a consistent tests section.
                body += "## Tests\n\n"
                tests = finding.get("tests") or []
                if not tests:
                    default_slug = f"test_{card_snake}_{next_num - 1:02d}"
                    default_scenario = (
                        finding.get("description", "").split(".")[0][:240]
                        or "See description above."
                    )
                    tests = [{"slug": default_slug, "scenario": default_scenario}]
                for t in tests:
                    body += f"### {t['slug']}\n"
                    body += "Source ticket: (new)\n"
                    body += "Implementation: (not yet written)\n"
                    body += f"Scenario: {t['scenario']}\n\n"
                body = body.rstrip() + "\n"

                fm = {
                    "id": ticket_id,
                    "status": "new",
                    "card": card,
                    "card_file": f"mtg-engine/src/cards/isd/{card_snake}.rs",
                    "created": now_iso(),
                    "audit_run_id": run_id,
                    "audit_model": args.model,
                    "audit_tokens": result["tokens"],
                    "audit_duration": result["duration"],
                }
                write_ticket(ticket_id, fm, body)
                tickets_created.append(ticket_id)

                # Log finding
                append_jsonl(METRICS_DIR / "findings.jsonl", {
                    "finding_id": ticket_id, "timestamp": now_iso(),
                    "event": "created", "card": card,
                    "source": "code-audit",
                    "engine_file": finding.get("engine_path", ""),
                    "description": desc_short,
                    "run_id": run_id,
                })

            # Append insights
            if parsed["insights"]:
                insights_file = PROMPTS_DIR / "auditor-insights.md"
                with open(insights_file, "a") as f:
                    for insight in parsed["insights"]:
                        f.write(f"\n{insight}\n")

            # Clean up staging
            staging_file.unlink()
        else:
            parsed = {"findings": [], "checks": {}, "is_pass": True}

        # Log run
        append_jsonl(METRICS_DIR / "runs.jsonl", {
            "run_id": run_id, "timestamp": now_iso(), "role": "auditor",
            "model": args.model, "card": card, "finding_id": None,
            "findings_created": len(tickets_created),
            "test_result": None, "fix_result": None,
            "validation_passed": True, "rejection_reason": None,
            "total_tokens": result["tokens"], "tool_uses": result["tool_uses"],
            "duration_seconds": result["duration"], "notes": "",
        })

        n = len(tickets_created)
        status = "PASS" if parsed["is_pass"] else f"{n} ticket(s)"
        print(f"  [{card}] Done: {status} ({result['duration']}s, {result['tokens']} tok)")
        return {"card": card, "tickets": n, "duration": result["duration"],
                "tokens": result["tokens"]}

    parallelism = args.parallelism
    cards_to_audit = [c for c in cards if c in card_oracles]

    if parallelism > 1:
        print(f"\nRunning {len(cards_to_audit)} audits (parallelism={parallelism})...")
        with concurrent.futures.ThreadPoolExecutor(max_workers=parallelism) as pool:
            futures = {pool.submit(audit_one, c): c for c in cards_to_audit}
            results = [f.result() for f in concurrent.futures.as_completed(futures)]
    else:
        results = [audit_one(c) for c in cards_to_audit]

    # Summary
    print(f"\n{'='*60}")
    print("AUDIT SUMMARY")
    print(f"{'='*60}")
    total = 0
    errors = []
    for r in sorted(results, key=lambda x: x["card"]):
        t = r["tickets"]
        total += t
        if r.get("error"):
            status = "ERROR"
            errors.append(r)
        else:
            status = "PASS" if t == 0 else f"{t} ticket(s)"
        print(f"  {r['card']:<30} {status:<15} {r['duration']}s  {r['tokens']} tok")
    print(f"\n  Total tickets created: {total}")
    if errors:
        print(f"\n  {len(errors)} agent error(s):")
        for r in errors:
            print(f"    {r['card']}: {r['error']}")


# ─── Test command ─────────────────────────────────────────────────

def cmd_test(args):
    if args.tickets:
        ids = [t.strip() for t in args.tickets.split(",")]
        tickets = [parse_ticket(TICKETS_DIR / f"{tid}.md")
                   for tid in ids if (TICKETS_DIR / f"{tid}.md").exists()]
    else:
        tickets = list_tickets(status="new")

    if not tickets:
        print("No tickets to test.")
        return

    print(f"\n{'='*60}")
    print(f"TEST WRITER — {len(tickets)} ticket(s)")
    print(f"{'='*60}")
    for t in tickets:
        fm = t["frontmatter"]
        print(f"  {fm.get('id', '?')}: {fm.get('card', '?')}")

    if args.dry_run:
        print("\n(dry run)")
        return

    shared_prompt = (PROMPTS_DIR / "test-writer.md").read_text()

    def test_one(ticket: dict) -> dict:
        fm = ticket["frontmatter"]
        tid = fm["id"]
        card = fm.get("card", "unknown")
        tid_snake = tid.replace("-", "_")

        oracle = get_oracle_text(card) or "Oracle text not available"

        # Create worktree for this ticket
        wt_dir = ensure_worktree(tid)
        wt_staging = wt_dir / "pipeline" / "staging"
        wt_staging.mkdir(parents=True, exist_ok=True)
        staging_file = wt_staging / f"{tid}-test.md"

        per_agent_base = f"""## Ticket to test

{ticket["body"]}

### Oracle text (pre-fetched from Scryfall, if available for a single card)

{oracle}

### Test file
Write every test for this ticket to a single file:
`mtg-engine/tests/pipeline_bugs_{tid_snake}.rs`

### Staging output
Write your result to: `pipeline/staging/{tid}-test.md`
Use the multi-test format specified in the shared prompt:
`## Test File`, then one `## Test: <slug>` block per entry in the
ticket's `## Tests` section (fields: Status, Test name, Assertion
message, Explanation, Blocked by).

### Commit your work
After all tests validate, commit the test file with a descriptive
message BEFORE writing the staging output. The worktree must be
clean (`git status --porcelain` empty). Python will reject the run
otherwise.

### Ticket ID: {tid}
"""

        # Retry loop: re-spawn the agent if validation fails. Typical
        # reason: agent forgot to commit its work, leaving the test
        # file untracked in the worktree.
        MAX_TEST_ATTEMPTS = 3
        parsed: dict = {"test_file": "", "tests": []}
        aggregate = "rejected"
        validated = False
        impls_by_slug: dict[str, str] = {}
        per_test: list = []
        test_file = f"mtg-engine/tests/pipeline_bugs_{tid_snake}.rs"
        retry_note = ""

        for attempt in range(1, MAX_TEST_ATTEMPTS + 1):
            print(f"  [{tid}] Spawning agent in worktree "
                  f"(attempt {attempt}/{MAX_TEST_ATTEMPTS})...")
            prompt = shared_prompt + "\n\n---\n\n" + per_agent_base + retry_note
            log_path = LOGS_DIR / f"{today()}-{tid}-test-attempt{attempt}.log"
            result = run_agent_in(prompt, wt_dir, args.model, args.effort,
                                  log_path=log_path,
                                  progress_prefix=f"  [{tid}] ")

            if result.get("is_error"):
                err = result.get("error_message") or "unknown agent error"
                print(f"  [{tid}] Agent error: {err}")
                retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                              f"Previous attempt errored: {err}\n")
                continue

            # Parse multi-test staging
            parsed = {"test_file": "", "tests": []}
            if staging_file.exists():
                parsed = parse_test_staging(staging_file)
                staging_file.unlink()

            test_file = parsed.get("test_file") or f"mtg-engine/tests/pipeline_bugs_{tid_snake}.rs"
            per_test = parsed.get("tests", [])

            if not per_test:
                retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                              f"Previous attempt produced no staging file or no "
                              f"`## Test: <slug>` blocks.\n")
                continue

            # Per-test validation
            impls_by_slug = {}
            for t in per_test:
                slug = t["slug"]
                status = t["status"]
                if status == "confirmed" and t["test_name"]:
                    test_path = wt_dir / test_file
                    if not test_path.exists():
                        t["status"] = "rejected"
                        impls_by_slug[slug] = "rejected: test file missing"
                        continue
                    val = subprocess.run(
                        [str(SCRIPTS_DIR / "validate_test.sh"),
                         str(test_path), t["test_name"]],
                        capture_output=True, text=True, cwd=str(wt_dir),
                    )
                    if val.returncode == 0:
                        impls_by_slug[slug] = f"{test_file}::{t['test_name']}"
                    else:
                        t["status"] = "rejected"
                        impls_by_slug[slug] = "rejected: validation failed"
                elif status == "rejected":
                    impls_by_slug[slug] = f"rejected: {t.get('explanation','')[:80]}"
                elif status == "blocked":
                    impls_by_slug[slug] = f"blocked: {t.get('blocked_by','')[:80]}"

            statuses = {t["status"] for t in per_test}
            if statuses == {"confirmed"}:
                aggregate = "confirmed"
            elif "blocked" in statuses:
                aggregate = "blocked"
            else:
                aggregate = "rejected"

            # Coverage check: every slug in the ticket's ## Tests section
            # must have a `confirmed` entry in the agent's output. Catches
            # the case where the agent silently omits a test.
            expected_slugs = {ct["slug"] for ct in _parse_tests_section(ticket["body"])}
            confirmed_slugs = {t["slug"] for t in per_test if t["status"] == "confirmed"}
            missing_slugs = expected_slugs - confirmed_slugs
            if missing_slugs:
                print(f"  [{tid}] Missing confirmed tests for slugs: "
                      f"{sorted(missing_slugs)}")
                aggregate = "rejected"
                if attempt < MAX_TEST_ATTEMPTS:
                    retry_note = (
                        f"\n\n## Retry note (attempt {attempt} failed)\n"
                        f"The ticket's `## Tests` section lists "
                        f"{len(expected_slugs)} test slug(s), but your "
                        f"staging output does not have a confirmed entry "
                        f"for every one. Missing: "
                        f"{sorted(missing_slugs)}. Every slug must have a "
                        f"corresponding failing Rust test.\n")
                    continue

            # Worktree-clean check: the agent must commit its work before
            # writing the staging output.
            if aggregate == "confirmed":
                git_status = subprocess.run(
                    ["git", "status", "--porcelain"],
                    capture_output=True, text=True, cwd=str(wt_dir),
                )
                dirty = git_status.stdout.strip()
                if dirty:
                    print(f"  [{tid}] Worktree dirty — agent forgot to commit:")
                    print(dirty)
                    aggregate = "rejected"
                    if attempt < MAX_TEST_ATTEMPTS:
                        retry_note = (
                            f"\n\n## Retry note (attempt {attempt} failed)\n"
                            f"Every per-test validation succeeded, but the "
                            f"worktree is not clean. `git status --porcelain` "
                            f"reported:\n\n{dirty}\n\n"
                            f"You must `git add -A && git commit` your test "
                            f"file before writing the staging output. Do this "
                            f"and try again.\n")
                        continue
            validated = (aggregate == "confirmed")
            break

        # Update ticket body: fill in Implementation for each test entry
        if impls_by_slug:
            update_tests_section_impls(tid, impls_by_slug)

        # If rejected/blocked overall, remove the worktree
        if aggregate in ("rejected", "blocked"):
            remove_worktree(tid)

        # Append a compact per-test result section to the ticket
        section_lines = ["## Test Run Results", ""]
        for t in per_test:
            section_lines.append(f"- **{t['slug']}** — {t['status']}")
            if t.get("test_name"):
                section_lines.append(f"  - test fn: `{t['test_name']}`")
            if t.get("assertion_message"):
                section_lines.append(f"  - assertion: {t['assertion_message']}")
            if t.get("blocked_by"):
                section_lines.append(f"  - blocked by: {t['blocked_by']}")
        append_ticket_section(tid, "\n".join(section_lines))

        # Update ticket status + metadata
        extra_fm = {
            f"{aggregate}_at": now_iso(),
            "test_run_id": f"{today()}-{tid}-test",
            "test_model": args.model,
            "test_tokens": str(result["tokens"]),
            "test_duration": str(result["duration"]),
            "test_file": test_file,
            "tests_confirmed": str(sum(1 for t in per_test if t["status"] == "confirmed")),
            "tests_total": str(len(per_test)),
        }
        if aggregate == "confirmed":
            extra_fm["worktree"] = str(get_worktree_dir(tid))
        update_ticket_status(tid, aggregate, extra_fm)

        # Log
        append_jsonl(METRICS_DIR / "runs.jsonl", {
            "run_id": f"{today()}-{tid}-test", "timestamp": now_iso(),
            "role": "test-writer", "model": args.model,
            "card": card, "finding_id": tid,
            "findings_created": 0, "test_result": aggregate,
            "fix_result": None, "validation_passed": validated,
            "rejection_reason": None if validated else "one-or-more tests failed validation",
            "total_tokens": result["tokens"], "tool_uses": result["tool_uses"],
            "duration_seconds": result["duration"],
            "notes": f"{extra_fm['tests_confirmed']}/{extra_fm['tests_total']} tests confirmed",
        })
        append_jsonl(METRICS_DIR / "findings.jsonl", {
            "finding_id": tid, "timestamp": now_iso(),
            "event": f"test_{aggregate}",
            "card": card, "source": "code-audit",
            "engine_file": "", "description": tid,
            "run_id": f"{today()}-{tid}-test",
            "test_file": test_file,
        })

        n_ok = extra_fm["tests_confirmed"]
        n_total = extra_fm["tests_total"]
        print(f"  [{tid}] Done: {aggregate} — {n_ok}/{n_total} tests "
              f"({result['duration']}s, {result['tokens']} tok)")
        return {"ticket": tid, "result": aggregate,
                "confirmed": int(n_ok), "total": int(n_total)}

    parallelism = args.parallelism
    if parallelism > 1:
        with concurrent.futures.ThreadPoolExecutor(max_workers=parallelism) as pool:
            futures = {pool.submit(test_one, t): t for t in tickets}
            results = [f.result() for f in concurrent.futures.as_completed(futures)]
    else:
        results = [test_one(t) for t in tickets]

    # Summary
    print(f"\n{'='*60}")
    print("TEST SUMMARY")
    print(f"{'='*60}")
    for r in sorted(results, key=lambda x: x["ticket"]):
        n_ok = r.get("confirmed", 0)
        n_total = r.get("total", 0)
        detail = f"{n_ok}/{n_total} tests" if n_total else ""
        print(f"  {r['ticket']:<40} {r['result']:<10} {detail}")
    confirmed = sum(1 for r in results if r["result"] == "confirmed")
    rejected = sum(1 for r in results if r["result"] == "rejected")
    blocked = sum(1 for r in results if r["result"] == "blocked")
    print(f"\n  Confirmed: {confirmed}  Rejected: {rejected}  Blocked: {blocked}")


# ─── Fix command ──────────────────────────────────────────────────

def cmd_fix(args):
    if args.ticket:
        path = TICKETS_DIR / f"{args.ticket}.md"
        if not path.exists():
            print(f"Ticket not found: {args.ticket}")
            sys.exit(1)
        tickets = [parse_ticket(path)]
    else:
        tickets = list_tickets(status="confirmed")[:1]

    if not tickets:
        print("No confirmed tickets to fix.")
        return

    ticket = tickets[0]
    fm = ticket["frontmatter"]
    tid = fm["id"]
    card = fm.get("card", "unknown")

    print(f"\n{'='*60}")
    print(f"FIXER — {tid} ({card})")
    print(f"{'='*60}")

    if args.dry_run:
        print("\n(dry run)")
        return

    shared_prompt = (PROMPTS_DIR / "fixer.md").read_text()

    # Reuse the ticket's worktree (created during test phase)
    wt_dir = get_worktree_dir(tid)
    if not wt_dir.exists():
        print(f"  No worktree found for {tid}. Run `test` first.")
        sys.exit(1)

    wt_staging = wt_dir / "pipeline" / "staging"
    wt_staging.mkdir(parents=True, exist_ok=True)
    staging_file = wt_staging / f"{tid}-fix.md"

    # Collect all tests (slug + implementation) from the ticket body.
    # Both single-test and multi-test tickets are structured identically
    # via the ## Tests section.
    ticket_tests = _parse_tests_section(ticket["body"])
    test_file = fm.get("test_file", "")
    test_fns = []
    for ct in ticket_tests:
        # Implementation line on a confirmed ticket looks like
        # "path/to/file.rs::fn_name". Only include fully-implemented tests.
        body_ct = ticket["body"]
        impl_match = re.search(
            rf"(?ms)^###\s+{re.escape(ct['slug'])}\s*$.*?^Implementation:\s*(.+?)$",
            body_ct)
        if impl_match:
            impl = impl_match.group(1).strip()
            if "::" in impl:
                test_fns.append(impl.split("::", 1)[1])

    failing_tests_block = "\n".join(
        f"- `{name}`" for name in test_fns) or "- (see ticket `## Tests` section)"

    per_agent_base = f"""## Ticket to fix

{ticket["body"]}

### Failing tests
This ticket has {len(test_fns)} test(s) that must ALL pass after your fix.
They all live in a single file:
- File: `{test_file}`
- Test functions:
{failing_tests_block}

### Staging output
Write your result to: `pipeline/staging/{tid}-fix.md`
Use the format: ## Status, ## Files Changed, ## Description

### Rules
- Only modify files under `mtg-engine/src/`
- Do NOT modify test files
- EVERY test listed above must pass after your fix
- Zero compiler warnings
- The full `cargo test` suite must still pass (no regressions)
- Commit your work (including the test file if untracked) before
  writing the staging output. The worktree must be clean when
  validate_fix.sh runs.
"""

    # Agent runs with a retry loop when validation fails (typically
    # because the agent forgot to commit its changes). On each retry
    # we append the validator's output so the agent knows what to fix.
    fix_result = "failed"
    validated = False
    parsed: dict = {}
    retry_note = ""
    MAX_FIX_ATTEMPTS = 3
    for attempt in range(1, MAX_FIX_ATTEMPTS + 1):
        print(f"  Spawning agent in worktree {wt_dir.name} "
              f"(attempt {attempt}/{MAX_FIX_ATTEMPTS})...")
        prompt = shared_prompt + "\n\n---\n\n" + per_agent_base + retry_note
        log_path = LOGS_DIR / f"{today()}-{tid}-fix-attempt{attempt}.log"
        result = run_agent_in(prompt, wt_dir, args.model, args.effort,
                              log_path=log_path,
                              progress_prefix=f"  [{tid}] ")

        if result.get("is_error"):
            err = result.get("error_message") or "unknown agent error"
            print(f"  Agent error: {err}")
            retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                          f"Previous attempt errored: {err}\n")
            continue

        if not staging_file.exists():
            retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                          f"Previous attempt did not write {staging_file}. "
                          f"Write the staging output there.\n")
            continue

        parsed = parse_fix_staging(staging_file)
        fix_result = parsed.get("status", "failed")
        staging_file.unlink()

        if fix_result != "fixed":
            # If the agent gave up, require a post-mortem. A `failed`
            # status with no description is useless — we lose the only
            # record of what went wrong. Retry with feedback demanding
            # a description.
            description = (parsed.get("description") or "").strip()
            if not description and attempt < MAX_FIX_ATTEMPTS:
                print(f"  Agent reported status={fix_result} with no "
                      f"Description — retrying to get a post-mortem")
                retry_note = (
                    f"\n\n## Retry note (attempt {attempt} failed)\n"
                    f"Previous attempt reported `status: failed` but "
                    f"omitted `## Description`. If you genuinely cannot "
                    f"fix this bug, the Description MUST explain what "
                    f"you tried, what failed, and what engine-level "
                    f"change would be required. That post-mortem is the "
                    f"single most useful artifact of a failed run — "
                    f"don't skip it.\n")
                continue
            print(f"  Agent reported status={fix_result}; not retrying")
            break

        # Validate — worktree must be clean + tests pass + no warnings.
        val = subprocess.run(
            [str(SCRIPTS_DIR / "validate_fix.sh")],
            capture_output=True, text=True, cwd=str(wt_dir),
        )
        validated = val.returncode == 0
        if validated:
            break

        # Validation failed — surface reason and retry
        tail = (val.stdout[-1500:] if val.stdout else "") + \
               (val.stderr[-500:] if val.stderr else "")
        print(f"  Validation FAILED (attempt {attempt}):")
        print(tail)
        fix_result = "failed"
        if attempt < MAX_FIX_ATTEMPTS:
            retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                          f"validate_fix.sh rejected your previous attempt. "
                          f"Output (last 1500 chars):\n\n{tail}\n\n"
                          f"Fix the issue and try again. Remember to commit "
                          f"your changes before running validate_fix.sh.\n")

    # Keep the worktree around on failure so humans can inspect the
    # agent's partial progress (uncommitted or on the fix branch) and
    # optionally resume work manually. `cli.py abandon` removes it
    # explicitly when we give up on the ticket.
    if fix_result == "failed":
        print(f"  Worktree preserved for inspection: {wt_dir}")
        print(f"  Run `./pipeline/cli.py abandon {tid}` to remove it.")

    # Build ticket section
    section = f"## Fix Result\n\n"
    section += f"status: {fix_result}\n"
    if parsed.get("files_changed"):
        section += f"files_changed: {parsed['files_changed']}\n"
    if parsed.get("description"):
        section += f"\n{parsed['description']}\n"

    append_ticket_section(tid, section)
    update_ticket_status(tid, fix_result, {
        f"{fix_result}_at": now_iso(),
        "fix_run_id": f"{today()}-{tid}-fix",
        "fix_model": args.model,
        "fix_tokens": str(result["tokens"]),
        "fix_duration": str(result["duration"]),
    })

    # Log
    append_jsonl(METRICS_DIR / "runs.jsonl", {
        "run_id": f"{today()}-{tid}-fix", "timestamp": now_iso(),
        "role": "fixer", "model": args.model,
        "card": card, "finding_id": tid,
        "findings_created": 0, "test_result": None,
        "fix_result": fix_result, "validation_passed": validated,
        "rejection_reason": None if validated else "validation failed",
        "total_tokens": result["tokens"], "tool_uses": result["tool_uses"],
        "duration_seconds": result["duration"], "notes": "",
    })
    append_jsonl(METRICS_DIR / "findings.jsonl", {
        "finding_id": tid, "timestamp": now_iso(),
        "event": "fix_succeeded" if fix_result == "fixed" else "fix_failed",
        "card": card, "source": "code-audit",
        "engine_file": "", "description": tid,
        "run_id": f"{today()}-{tid}-fix",
    })

    print(f"\n  Result: {fix_result} ({result['duration']}s, {result['tokens']} tok)")


# ─── Tickets command ──────────────────────────────────────────────

def cmd_tickets(args):
    tickets = list_tickets(status=args.status, card=args.card)

    if not tickets:
        print("No tickets found.")
        return

    # Group by status
    by_status = {}
    for t in tickets:
        s = t["frontmatter"].get("status", "unknown")
        by_status.setdefault(s, []).append(t)

    # Primary statuses display in a fixed order; any remaining statuses
    # (e.g. "deduped", or newly-introduced states) display afterwards.
    primary = ["new", "confirmed", "blocked", "fixed", "failed", "rejected", "merged"]
    remainder = [s for s in by_status if s not in primary]
    for status in primary + sorted(remainder):
        group = by_status.get(status, [])
        if not group:
            continue
        print(f"\n{status.upper()} ({len(group)})")
        for t in group:
            fm = t["frontmatter"]
            card = fm.get("card", "?")
            tid = fm.get("id", "?")
            extra = ""
            target = fm.get("duplicate_of") or fm.get("deduped_into")
            if target:
                extra = f" → {target}"
            print(f"  {tid:<30} {card}{extra}")


# ─── Show command ─────────────────────────────────────────────────

def cmd_show(args):
    path = TICKETS_DIR / f"{args.ticket_id}.md"
    if not path.exists():
        print(f"Ticket not found: {args.ticket_id}")
        sys.exit(1)
    print(path.read_text())


# ─── Accept command ───────────────────────────────────────────────

def cmd_merge(args):
    # Collect tickets to merge
    if args.ticket_id == "all":
        tickets = list_tickets(status="fixed")
    else:
        ids = [t.strip() for t in args.ticket_id.split(",")]
        tickets = []
        for tid in ids:
            path = TICKETS_DIR / f"{tid}.md"
            if path.exists():
                t = parse_ticket(path)
                if t["frontmatter"].get("status") == "fixed":
                    tickets.append(t)
                else:
                    print(f"  Skipping {tid}: status is {t['frontmatter'].get('status')}, not fixed")
            else:
                print(f"  Skipping {tid}: not found")

    if not tickets:
        print("No fixed tickets to merge.")
        return

    print(f"\n{'='*60}")
    print(f"MERGE — {len(tickets)} ticket(s)")
    print(f"{'='*60}")
    for t in tickets:
        fm = t["frontmatter"]
        print(f"  {fm['id']:<30} {fm.get('card', '?')}")

    if args.dry_run:
        print("\n(dry run)")
        return

    for ticket in tickets:
        fm = ticket["frontmatter"]
        tid = fm["id"]
        test_name = fm.get("test_name", "")
        wt_dir = get_worktree_dir(tid)
        branch = get_worktree_branch(tid)

        print(f"\n  [{tid}] Merging...")

        if not wt_dir.exists():
            print(f"  [{tid}] No worktree found — skipping")
            continue

        # Merge the branch
        merge_result = subprocess.run(
            ["git", "merge", branch, "--no-edit"],
            capture_output=True, text=True, cwd=str(PROJECT_ROOT),
        )
        if merge_result.returncode != 0:
            print(f"  [{tid}] Merge FAILED: {merge_result.stderr[:200]}")
            continue

        # Verify the test exists at HEAD and passes
        if test_name:
            test_check = subprocess.run(
                ["cargo", "test", "--", test_name],
                capture_output=True, text=True, cwd=str(PROJECT_ROOT),
                timeout=300,
            )
            if "FAILED" in test_check.stdout or test_check.returncode != 0:
                print(f"  [{tid}] Test fails at HEAD after merge — reverting")
                subprocess.run(["git", "reset", "--hard", "HEAD~1"],
                             capture_output=True, cwd=str(PROJECT_ROOT))
                continue
            print(f"  [{tid}] Test passes at HEAD")

        # Capture the merge commit sha before cleaning up so the ticket
        # points at something durable instead of a worktree that's about
        # to be deleted.
        merge_sha = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True, text=True, cwd=str(PROJECT_ROOT),
        ).stdout.strip()

        # Clean up worktree + any agent output logs associated with
        # this ticket (they were only useful mid-flight / for
        # post-mortem; a shipped fix means we won't replay them).
        remove_worktree(tid)
        n_logs = remove_logs_for_ticket(tid)
        if n_logs:
            print(f"  [{tid}] Removed {n_logs} agent log file(s)")

        # Update ticket — drop the now-stale worktree field and record
        # the merge commit in its place.
        path = TICKETS_DIR / f"{tid}.md"
        t = parse_ticket(path)
        t["frontmatter"].pop("worktree", None)
        t["frontmatter"]["status"] = "merged"
        t["frontmatter"]["merged_at"] = now_iso()
        if merge_sha:
            t["frontmatter"]["merged_sha"] = merge_sha
        write_ticket(tid, t["frontmatter"], t["body"])
        append_jsonl(METRICS_DIR / "findings.jsonl", {
            "finding_id": tid, "timestamp": now_iso(),
            "event": "merged", "card": fm.get("card", ""),
            "source": "code-audit", "engine_file": "",
            "description": tid, "run_id": "merge",
        })
        print(f"  [{tid}] Merged and cleaned up")


# ─── Abandon command ──────────────────────────────────────────────

def cmd_retry(args):
    """Retry a failed ticket's most-recent stage.

    Looks at the ticket's frontmatter to decide what to re-run:
    - If the ticket has `fix_run_id` set (a fix was attempted), reset
      status to `confirmed` and re-run `cli.py fix`. Reuses the
      existing worktree if present; otherwise recreates it from the
      ticket's fix branch (assumed to contain the test commits).
    - Otherwise (test stage failed), reset status to `new` and
      re-run `cli.py test`. A stale worktree is removed first so the
      test-writer starts fresh.

    Only acts on tickets currently in `status: failed` or `blocked`
    — refuses on `new`/`confirmed`/`fixed`/`merged`/`closed-duplicate`.
    """
    tid = args.ticket_id
    path = TICKETS_DIR / f"{tid}.md"
    if not path.exists():
        print(f"Ticket not found: {tid}")
        sys.exit(1)
    ticket = parse_ticket(path)
    fm = ticket["frontmatter"]
    status = fm.get("status", "")
    if status not in ("failed", "blocked"):
        print(f"ERROR: {tid} is status={status}; retry only works on "
              f"'failed' or 'blocked' tickets")
        sys.exit(1)

    had_fix = bool(fm.get("fix_run_id"))
    wt_dir = get_worktree_dir(tid)
    branch = get_worktree_branch(tid)

    if had_fix:
        print(f"[{tid}] Last stage was fix — resetting to confirmed and "
              f"re-running fixer")
        if not wt_dir.exists():
            # Try to re-attach the fix branch to a new worktree. The
            # branch should still exist (cmd_fix preserves it) unless
            # cli.py abandon was run; if missing, the user needs to
            # recover manually (cherry-pick the test commits).
            branch_check = subprocess.run(
                ["git", "rev-parse", "--verify", branch],
                capture_output=True, cwd=str(PROJECT_ROOT),
            )
            if branch_check.returncode != 0:
                print(f"ERROR: worktree gone and branch {branch} no "
                      f"longer exists. Manual recovery required — "
                      f"cherry-pick the test commits onto a new branch.")
                sys.exit(1)
            print(f"  Re-attaching worktree to existing branch {branch}")
            subprocess.run(
                ["git", "worktree", "add", str(wt_dir), branch],
                check=True, cwd=str(PROJECT_ROOT),
            )
        update_ticket_status(tid, "confirmed")
        # Delegate to cmd_fix with the same model/effort
        fix_args = argparse.Namespace(
            ticket=tid, model=args.model, effort=args.effort, dry_run=False,
        )
        cmd_fix(fix_args)
        return

    print(f"[{tid}] No fix_run_id recorded — treating as test-stage failure")
    # Start fresh: nuke any stale worktree
    if wt_dir.exists():
        print(f"  Removing stale worktree before fresh test run")
        remove_worktree(tid)
    update_ticket_status(tid, "new")
    test_args = argparse.Namespace(
        tickets=tid, parallelism=1,
        model=args.model, effort=args.effort, dry_run=False,
    )
    cmd_test(test_args)


def cmd_abandon(args):
    tid = args.ticket_id
    path = TICKETS_DIR / f"{tid}.md"
    if not path.exists():
        print(f"Ticket not found: {tid}")
        sys.exit(1)
    wt_dir = get_worktree_dir(tid)
    if wt_dir.exists():
        remove_worktree(tid)
        print(f"Removed worktree for {tid}")
    else:
        print(f"No worktree for {tid}")
    n_logs = remove_logs_for_ticket(tid)
    if n_logs:
        print(f"Removed {n_logs} agent log file(s) for {tid}")
    # Reset status back to new so it can be re-tested
    ticket = parse_ticket(path)
    status = ticket["frontmatter"].get("status", "")
    if status in ("confirmed", "fixed", "failed"):
        update_ticket_status(tid, "new")
        print(f"Reset {tid} status to new")


# ─── Status command ───────────────────────────────────────────────

def cmd_status(args):
    subprocess.run(["python3", str(SCRIPTS_DIR / "metrics.py")],
                   cwd=str(PROJECT_ROOT))


# ─── Report command ───────────────────────────────────────────────

def _load_audit_runs() -> list[dict]:
    path = METRICS_DIR / "runs.jsonl"
    if not path.exists():
        return []
    runs = []
    for line in path.read_text().strip().split("\n"):
        if not line:
            continue
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        if d.get("role") == "auditor":
            runs.append(d)
    return runs


def cmd_report(args):
    """Coverage + per-card breakdown crossing audit runs against ticket state."""
    from collections import Counter, defaultdict

    runs = _load_audit_runs()
    tickets = list_tickets()

    # ── Audit coverage section ──────────────────────────────────
    successful_runs = [r for r in runs if r.get("validation_passed")]
    errored_runs = [r for r in runs if not r.get("validation_passed")]
    cards_attempted = sorted({r["card"] for r in runs})
    cards_successful = sorted({r["card"] for r in successful_runs})
    cards_errored_only = sorted(set(cards_attempted) - set(cards_successful))

    # ── Per-card ticket state ──────────────────────────────────
    by_card_runs = Counter(r["card"] for r in successful_runs)
    by_card_status: dict[str, Counter] = defaultdict(Counter)
    by_card_total_tickets: Counter = Counter()
    for t in tickets:
        fm = t["frontmatter"]
        card = fm.get("card", "(unknown)")
        status = fm.get("status", "new")
        # Skip merged-* tickets when attributing to a card — they cover
        # multiple cards and their `card:` frontmatter is "multiple".
        if card == "multiple":
            continue
        by_card_status[card][status] += 1
        by_card_total_tickets[card] += 1

    # Count merged-* parent tickets separately
    merged_parents = [t for t in tickets
                      if t["frontmatter"].get("card") == "multiple"]

    # ── Print audit coverage ──────────────────────────────────
    if not args.cards_only:
        print(f"\n{'='*60}")
        print("AUDIT COVERAGE")
        print(f"{'='*60}")
        print(f"  Total audit runs:                  {len(runs)}")
        print(f"  Successful runs:                   {len(successful_runs)}")
        print(f"  Errored runs:                      {len(errored_runs)}")
        print(f"  Unique cards attempted:            {len(cards_attempted)}")
        print(f"  Unique cards successfully audited: {len(cards_successful)}")
        if cards_errored_only:
            print(f"\n  Cards with only errored runs ({len(cards_errored_only)}):")
            for c in cards_errored_only:
                errs = [r for r in errored_runs if r["card"] == c]
                reason = errs[-1].get("rejection_reason", "") or "unknown"
                print(f"    {c} — {reason}")

    # ── Print per-card breakdown ──────────────────────────────
    if not args.audits_only:
        # Resolve each ticket's terminal status by walking duplicate_of.
        # A card ticket whose parent chain ends in a merged/fixed ticket
        # is "effectively fixed" for counting purposes, even though its
        # own status stays closed-duplicate.
        OPEN_STATUSES = {"new", "confirmed", "blocked", "failed", "rejected"}
        FIXED_STATUSES = {"fixed", "merged"}
        CLOSED_DUP_STATUSES = {"closed-duplicate", "deduped", "duplicate"}

        by_id = {t["frontmatter"].get("id"): t for t in tickets}
        def _terminal_status(tid: str, _seen=None) -> str:
            _seen = _seen or set()
            if tid in _seen:
                return "(cycle)"
            _seen.add(tid)
            t = by_id.get(tid)
            if not t:
                return "(unknown)"
            st = t["frontmatter"].get("status", "new")
            if st not in CLOSED_DUP_STATUSES:
                return st
            parent = t["frontmatter"].get("duplicate_of") \
                or t["frontmatter"].get("deduped_into")
            if not parent:
                return st
            return _terminal_status(parent, _seen)

        print(f"\n{'='*60}")
        print("PER-CARD BREAKDOWN")
        print(f"{'='*60}")
        print(f"  {'Card':<32} {'Audits':>6} {'Tickets':>7} "
              f"{'Open':>5} {'Fixed':>5} {'Closed':>6}")
        print(f"  {'-'*32} {'-'*6} {'-'*7} {'-'*5} {'-'*5} {'-'*6}")

        rows = []
        for card in sorted(set(cards_attempted) | set(by_card_status)):
            audits = by_card_runs.get(card, 0)
            card_tickets = [t for t in tickets
                            if t["frontmatter"].get("card") == card]
            open_n = fixed_n = closed_open_n = 0
            for t in card_tickets:
                tid = t["frontmatter"].get("id")
                st = t["frontmatter"].get("status", "new")
                if st in OPEN_STATUSES:
                    open_n += 1
                elif st in FIXED_STATUSES:
                    fixed_n += 1
                elif st in CLOSED_DUP_STATUSES:
                    terminal = _terminal_status(tid)
                    if terminal in FIXED_STATUSES:
                        fixed_n += 1
                    else:
                        closed_open_n += 1
            total_tickets = len(card_tickets)
            rows.append((card, audits, total_tickets, open_n,
                         fixed_n, closed_open_n))

        # Sort: most tickets first, then by name
        rows.sort(key=lambda r: (-r[2], r[0]))
        for card, audits, total, open_n, fixed_n, closed_n in rows:
            print(f"  {card:<32} {audits:>6} {total:>7} "
                  f"{open_n:>5} {fixed_n:>5} {closed_n:>6}")

        clean = sum(1 for r in rows if r[1] > 0 and r[2] == 0)
        print(f"\n  {len(rows)} cards total — {clean} clean "
              f"(Audits>0, Tickets=0)")

    # ── Ticket backlog summary ───────────────────────────────
    if not args.cards_only and not args.audits_only:
        print(f"\n{'='*60}")
        print("TICKET BACKLOG")
        print(f"{'='*60}")
        status_counts = Counter(t["frontmatter"].get("status", "new")
                                for t in tickets)
        primary = ["new", "confirmed", "blocked", "fixed",
                   "failed", "rejected", "merged"]
        other = sorted(s for s in status_counts if s not in primary)
        for s in primary + other:
            n = status_counts.get(s, 0)
            if n:
                print(f"  {s:<12} {n:>4}")
        print(f"\n  Merged (parent) tickets: {len(merged_parents)}")
    print()


# ─── Consolidate command ─────────────────────────────────────────

def parse_consolidation_input(text: str) -> dict:
    """Parse a consolidation input file.

    Expected format:

        ---
        slug: <short-slug>
        ---

        # <Title>

        ## Description
        <multi-line>

        ## Engine path
        <multi-line, optional>

        ## Tests

        ### <test_slug_1>
        Source ticket: <ticket-id-or-omitted>
        Scenario: <multi-line>

        ### <test_slug_2>
        Source ticket: <ticket-id>
        Scenario: <multi-line>

    Returns dict with: slug, title, description, engine_path (optional),
    tests (list of {slug, source_ticket, scenario}).
    """
    if not text.startswith("---"):
        raise ValueError("consolidation file must start with --- frontmatter")
    try:
        end = text.index("---", 3)
    except ValueError:
        raise ValueError("missing closing --- of frontmatter")
    fm = {}
    for line in text[3:end].strip().split("\n"):
        if ":" in line:
            k, v = line.split(":", 1)
            fm[k.strip()] = v.strip()
    body = text[end + 3:].strip()

    slug = fm.get("slug")
    if not slug or not re.fullmatch(r"[a-z0-9][a-z0-9-]*", slug):
        raise ValueError(f"frontmatter 'slug' must be lowercase-kebab-case, got {slug!r}")

    # `## Also closes` (optional): bullet list of ticket ids that should be
    # closed-duplicate → new parent but do not contribute a unique test
    # entry. Typical use: when nesting an existing merged-* ticket, its
    # tests are copied into the new parent (each keeping its original card
    # Source ticket); the merged-* itself is listed here so it gets closed.
    also_closes: list[str] = []
    also_match = re.search(
        r"##\s+Also closes\n(.*?)(?=\n##\s|\Z)", body, re.DOTALL)
    if also_match:
        for line in also_match.group(1).strip().split("\n"):
            line = line.strip().lstrip("-").strip()
            if line:
                also_closes.append(line)

    title_match = re.match(r"#\s+(.+?)\n", body + "\n")
    if not title_match:
        raise ValueError("body must start with a '# <title>' heading")
    title = title_match.group(1).strip()

    def section(name: str, required: bool = False) -> str | None:
        m = re.search(
            rf"##\s+{re.escape(name)}\n(.*?)(?=\n##\s|\Z)",
            body, re.DOTALL)
        if not m:
            if required:
                raise ValueError(f"missing required '## {name}' section")
            return None
        return m.group(1).strip()

    description = section("Description", required=True)
    engine_path = section("Engine path")

    tests_section = section("Tests", required=True)
    tests = []
    # Each test starts with '### slug'
    for block in re.split(r"\n(?=###\s+)", tests_section):
        block = block.strip()
        if not block.startswith("###"):
            continue
        head, _, rest = block.partition("\n")
        test_slug = head[3:].strip()
        if not re.fullmatch(r"[a-z0-9_][a-z0-9_]*", test_slug):
            raise ValueError(f"test slug must be snake_case, got {test_slug!r}")
        source = None
        scenario_lines = []
        in_scenario = False
        for line in rest.split("\n"):
            if in_scenario:
                scenario_lines.append(line)
                continue
            if line.startswith("Source ticket:"):
                val = line.split(":", 1)[1].strip()
                source = val if val and val.lower() not in ("none", "null", "(new)", "") else None
            elif line.startswith("Scenario:"):
                in_scenario = True
                rest_of_line = line.split(":", 1)[1].strip()
                if rest_of_line:
                    scenario_lines.append(rest_of_line)
        scenario = "\n".join(scenario_lines).strip()
        if not scenario:
            raise ValueError(f"test {test_slug!r} missing Scenario")
        tests.append({"slug": test_slug, "source_ticket": source, "scenario": scenario})
    if not tests:
        raise ValueError("## Tests section must contain at least one ### entry")

    return {
        "slug": slug, "title": title, "description": description,
        "engine_path": engine_path, "tests": tests,
        "also_closes": also_closes,
    }


def _parse_tests_section(body: str) -> list[dict]:
    """Extract test entries {slug, source_ticket} from a ticket body's
    `## Tests` section. Used by consolidate to validate that a new parent
    covers every test from every ticket it's closing."""
    tests_match = re.search(r"##\s+Tests\n(.*?)(?=\n##\s|\Z)", body, re.DOTALL)
    if not tests_match:
        return []
    results = []
    for block in re.split(r"\n(?=###\s+)", tests_match.group(1)):
        block = block.strip()
        if not block.startswith("###"):
            continue
        head, _, rest = block.partition("\n")
        slug = head[3:].strip()
        source_match = re.search(r"Source ticket:\s*(.+)", rest)
        source = source_match.group(1).strip() if source_match else None
        results.append({"slug": slug, "source_ticket": source})
    return results


def render_consolidated_body(parsed: dict) -> str:
    lines = [f"# {parsed['title']}", "", "## Description", parsed["description"]]
    if parsed.get("engine_path"):
        lines += ["", "## Engine path", parsed["engine_path"]]
    lines += ["", "## Tests", ""]
    for t in parsed["tests"]:
        lines.append(f"### {t['slug']}")
        lines.append(f"Source ticket: {t['source_ticket'] or '(new)'}")
        lines.append("Implementation: (not yet written)")
        lines.append(f"Scenario: {t['scenario']}")
        lines.append("")
    if parsed.get("also_closes"):
        lines += ["## Also closes", ""]
        for tid in parsed["also_closes"]:
            lines.append(f"- {tid}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def cmd_consolidate(args):
    input_path = Path(args.input)
    if not input_path.exists():
        print(f"ERROR: input file not found: {input_path}")
        sys.exit(1)

    parsed = parse_consolidation_input(input_path.read_text())
    slug = parsed["slug"]

    # Collect source ticket ids (from per-test Source ticket:) plus
    # ## Also closes entries. Both get closed-duplicate → new parent; the
    # difference is cosmetic (Also closes entries don't carry a test).
    # Tickets may legitimately contribute multiple tests, so dedupe.
    test_source_ids_unique: list[str] = []
    seen = set()
    for t in parsed["tests"]:
        sid = t.get("source_ticket")
        if sid and sid not in seen:
            test_source_ids_unique.append(sid)
            seen.add(sid)
    also_closes = parsed.get("also_closes", [])

    overlap = set(test_source_ids_unique) & set(also_closes)
    if overlap:
        print(f"ERROR: ticket(s) listed both as Source ticket and in Also closes: {overlap}")
        sys.exit(1)

    all_closed_ids = list(test_source_ids_unique) + list(also_closes)
    if len(all_closed_ids) != len(set(all_closed_ids)):
        dupes = [x for x in all_closed_ids if all_closed_ids.count(x) > 1]
        print(f"ERROR: ticket appears multiple times in Also closes: {set(dupes)}")
        sys.exit(1)

    missing = [tid for tid in all_closed_ids
               if not (TICKETS_DIR / f"{tid}.md").exists()]
    if missing:
        print(f"ERROR: referenced ticket(s) not found: {missing}")
        sys.exit(1)

    # Test-coverage invariant: for every ticket being closed, the new
    # parent's `## Tests` section must contain at least as many tests
    # attributable to each of its Source tickets as the closed ticket
    # did. Slugs may be renamed by the dedup agent (generic audit
    # slugs are commonly specialized for clarity), so we validate by
    # Source-ticket *counts* rather than slug set.
    from collections import Counter
    parent_source_counts: Counter = Counter()
    for t in parsed["tests"]:
        sid = t.get("source_ticket")
        if sid:
            parent_source_counts[sid] += 1

    def _required_counts(child_body: str, child_tid: str) -> Counter:
        """Per-Source-ticket counts the parent must match, for a closed
        child ticket. Tests without an explicit Source ticket (`(new)`)
        are treated as needing a parent test with Source ticket=child_tid."""
        counts: Counter = Counter()
        for ct in _parse_tests_section(child_body):
            sid = (ct.get("source_ticket") or "").strip()
            if not sid or sid.lower() in ("(new)", "none", "null"):
                sid = child_tid
            counts[sid] += 1
        return counts

    coverage_gaps: list[str] = []
    for tid in all_closed_ids:
        child_body = parse_ticket(TICKETS_DIR / f"{tid}.md")["body"]
        required = _required_counts(child_body, tid)
        for source, need in required.items():
            have = parent_source_counts.get(source, 0)
            if have < need:
                coverage_gaps.append(
                    f"  {tid}: needs ≥ {need} test(s) with Source ticket "
                    f"'{source}' in parent, found {have}")
    if coverage_gaps:
        print("ERROR: new parent is missing tests that exist on closed tickets.")
        print("For each closed ticket, the parent must have at least as many")
        print("tests attributable to each Source ticket as the closed ticket did.")
        for gap in coverage_gaps:
            print(gap)
        sys.exit(1)

    # Mint new id: merged-<slug>-NN
    existing = sorted(TICKETS_DIR.glob(f"merged-{slug}-*.md"))
    nums = []
    for f in existing:
        m = re.search(rf"merged-{re.escape(slug)}-(\d+)", f.stem)
        if m:
            nums.append(int(m.group(1)))
    next_num = max(nums, default=0) + 1
    new_id = f"merged-{slug}-{next_num:02d}"

    if args.dry_run:
        print(f"\nWould create: {new_id}")
        print(f"Would close {len(all_closed_ids)} ticket(s) as closed-duplicate:")
        for tid in all_closed_ids:
            print(f"  {tid} → {new_id}")
        return

    # Write new ticket
    fm = {
        "id": new_id,
        "status": "new",
        "card": "multiple",
        "created": now_iso(),
        "kind": "consolidated",
    }
    if all_closed_ids:
        fm["source_tickets"] = ", ".join(all_closed_ids)
    body = render_consolidated_body(parsed)
    write_ticket(new_id, fm, body)

    # Close every referenced ticket (per-test sources + also_closes items).
    for tid in all_closed_ids:
        path = TICKETS_DIR / f"{tid}.md"
        ticket = parse_ticket(path)
        ticket["frontmatter"]["status"] = "closed-duplicate"
        ticket["frontmatter"]["duplicate_of"] = new_id
        # Remove any legacy fields from earlier versions of this pipeline.
        ticket["frontmatter"].pop("deduped_into", None)
        write_ticket(tid, ticket["frontmatter"], ticket["body"])

    print(f"Created {new_id} with {len(parsed['tests'])} test(s)")
    print(f"Marked {len(all_closed_ids)} ticket(s) as status=closed-duplicate "
          f"({len(test_source_ids_unique)} per-test, {len(also_closes)} via Also closes)")

    # Staging files are ephemeral transport; remove once consumed successfully.
    if not args.keep_input:
        try:
            input_path.unlink()
            print(f"Removed staging file: {input_path}")
        except OSError as e:
            print(f"WARNING: could not remove staging file {input_path}: {e}")


# ─── Dedup command ──────────────────────────────────────────────

DEDUP_MAX_ATTEMPTS = 3


_CLOSED_STATUSES = {"closed-duplicate", "fixed", "merged",
                    "deduped", "duplicate"}  # last two for legacy tickets


def cmd_dedup(args):
    """Spawn a dedup agent to consider a set of candidate tickets for dedup.

    The passed tickets are a *seed* — the agent may include, exclude, or
    extend beyond them by searching `pipeline/tickets/` directly. The
    agent may produce zero or more consolidation staging files (one per
    proposed cluster). Python then runs `consolidate` on each.

    merged-* tickets are valid sources; the resulting parent nests them
    and must inherit every test from each child's `## Tests` section.

    If the agent's output fails to parse, the agent is re-spawned with
    the parser error appended, up to DEDUP_MAX_ATTEMPTS times.
    """
    ticket_ids = [t.strip() for t in args.tickets.split(",") if t.strip()]
    if not ticket_ids:
        print("ERROR: dedup requires at least one candidate ticket id")
        sys.exit(1)

    ticket_bodies = []
    for tid in ticket_ids:
        path = TICKETS_DIR / f"{tid}.md"
        if not path.exists():
            print(f"ERROR: ticket not found: {tid}")
            sys.exit(1)
        t = parse_ticket(path)
        status = t["frontmatter"].get("status", "new")
        if status in _CLOSED_STATUSES:
            print(f"ERROR: {tid} already closed (status={status})")
            sys.exit(1)
        ticket_bodies.append((tid, path.read_text()))

    shared_prompt = (PROMPTS_DIR / "dedup.md").read_text()
    tickets_section = "\n\n---\n\n".join(
        f"## Candidate ticket: {tid}\n\n{body}" for tid, body in ticket_bodies
    )

    per_agent_base = f"""## Candidate tickets (seed set)

The following {len(ticket_ids)} ticket(s) are proposed as the starting
point for your dedup analysis. You are NOT required to merge every one
of them, and you SHOULD search the full `pipeline/tickets/` directory
for other open tickets (card tickets or existing `merged-*` tickets)
that belong in the same clusters.

{tickets_section}

### Output
Write one consolidation file per proposed merged ticket to
`pipeline/staging/consolidation-<slug>.md` using the format in the
shared prompt. If no tickets should be merged, produce zero files.
Each file's frontmatter `slug:` must be distinct.
"""

    if args.dry_run:
        print(f"[dry-run] Would spawn dedup agent for {len(ticket_ids)} candidate ticket(s) "
              f"(model={args.model})")
        return

    def _staging_files() -> list[Path]:
        return sorted(STAGING_DIR.glob("consolidation-*.md"))

    # Snapshot pre-existing staging files so we only process what the
    # agent produces this run.
    preexisting = {f.name for f in _staging_files()}

    retry_note = ""
    last_error = None
    for attempt in range(1, DEDUP_MAX_ATTEMPTS + 1):
        print(f"\nSpawning dedup agent (attempt {attempt}/{DEDUP_MAX_ATTEMPTS})...")
        prompt = shared_prompt + "\n\n---\n\n" + per_agent_base + retry_note
        log_path = LOGS_DIR / f"{today()}-dedup-attempt{attempt}.log"
        result = run_agent(prompt, args.model, args.effort,
                           log_path=log_path,
                           progress_prefix="  [dedup] ")

        if result.get("is_error"):
            last_error = result.get("error_message") or "unknown agent error"
            print(f"  Agent error: {last_error} ({result['duration']}s)")
            retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                          f"Previous attempt errored: {last_error}\n")
            continue

        new_files = [f for f in _staging_files() if f.name not in preexisting]
        if not new_files:
            print(f"  Agent produced no consolidation files — nothing to merge "
                  f"({result['duration']}s, {result['tokens']} tok)")
            return

        # Validate every new staging file before ingesting any of them.
        parse_errors: list[tuple[Path, str]] = []
        for f in new_files:
            try:
                parse_consolidation_input(f.read_text())
            except ValueError as e:
                parse_errors.append((f, str(e)))

        if parse_errors:
            last_error = "; ".join(f"{f.name}: {msg}" for f, msg in parse_errors)
            print(f"  Parse errors in {len(parse_errors)} file(s) "
                  f"({result['duration']}s, {result['tokens']} tok)")
            for f, msg in parse_errors:
                print(f"    {f.name}: {msg}")
            errdetail = "\n".join(f"- {f.name}: {msg}" for f, msg in parse_errors)
            retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                          f"Previous attempt produced {len(parse_errors)} "
                          f"unparseable staging file(s):\n{errdetail}\n"
                          f"Edit the files in place to fix.\n")
            continue

        # All parsed — consolidate each
        print(f"  Agent produced {len(new_files)} valid staging file(s) "
              f"({result['duration']}s, {result['tokens']} tok)")
        for f in new_files:
            print(f"\n── consolidating {f.name} ──")
            consolidate_args = argparse.Namespace(
                input=str(f),
                keep_input=args.keep_input,
                dry_run=False,
            )
            cmd_consolidate(consolidate_args)
        return

    print(f"\nERROR: dedup failed after {DEDUP_MAX_ATTEMPTS} attempt(s). "
          f"Last error: {last_error}")
    sys.exit(1)


# ─── Close-duplicate command ────────────────────────────────────

def cmd_close_duplicate(args):
    """Mark a ticket as closed because it duplicates a bug tracked elsewhere.

    Unlike `consolidate`, this closes a single ticket without creating a new
    merged-* ticket. Use it when the bug is already tracked (existing test,
    bug ID in another system, etc.).
    """
    ticket_id = args.ticket
    path = TICKETS_DIR / f"{ticket_id}.md"
    if not path.exists():
        print(f"ERROR: ticket not found: {ticket_id}")
        sys.exit(1)

    ticket = parse_ticket(path)
    fm = ticket["frontmatter"]
    fm["status"] = "closed-duplicate"
    fm["duplicate_of"] = args.duplicate_of
    if args.reason:
        fm["duplicate_reason"] = args.reason
    write_ticket(ticket_id, fm, ticket["body"])
    print(f"Marked {ticket_id} as status=closed-duplicate → {args.duplicate_of}")


# ─── Main ─────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Pipeline CLI — manage bug-finding and fixing agents",
        formatter_class=argparse.RawDescriptionHelpFormatter)

    sub = parser.add_subparsers(dest="command", required=True)

    def add_common(p):
        p.add_argument("--model", default=DEFAULT_MODEL)
        p.add_argument("--effort", default=DEFAULT_EFFORT)
        p.add_argument("--dry-run", action="store_true")

    # audit
    p = sub.add_parser("audit", help="Audit cards for bugs")
    p.add_argument("--cards", required=True, help="Card names separated by ',' (or ';' if any name contains a comma, e.g. 'Mikaeus, the Lunarch')")
    p.add_argument("--parallelism", type=int, default=1)
    add_common(p)

    # test
    p = sub.add_parser("test", help="Write tests for new tickets")
    p.add_argument("--tickets", help="Comma-separated ticket IDs")
    p.add_argument("--parallelism", type=int, default=1)
    add_common(p)

    # fix
    p = sub.add_parser("fix", help="Fix a confirmed ticket")
    p.add_argument("--ticket", help="Specific ticket ID")
    add_common(p)

    # tickets
    p = sub.add_parser("tickets", help="List tickets")
    p.add_argument("--status", help="Filter by status")
    p.add_argument("--card", help="Filter by card name")

    # show
    p = sub.add_parser("show", help="Display a ticket")
    p.add_argument("ticket_id", help="Ticket ID")

    # merge
    p = sub.add_parser("merge", help="Merge fixed ticket(s) to HEAD")
    p.add_argument("ticket_id", help="Ticket ID, comma-separated IDs, or 'all'")
    add_common(p)

    # abandon
    p = sub.add_parser("abandon", help="Remove worktree for a ticket without merging")
    p.add_argument("ticket_id", help="Ticket ID")

    # retry
    p = sub.add_parser("retry",
                       help="Re-run the failed stage of a ticket (fix or test) — resets status and re-spawns the agent")
    p.add_argument("ticket_id", help="Ticket ID")
    add_common(p)

    # status
    sub.add_parser("status", help="Show metrics dashboard")

    # report
    p = sub.add_parser("report",
                       help="Audit coverage, per-card breakdown, and ticket backlog")
    p.add_argument("--audits-only", action="store_true",
                   help="Show only the audit coverage section")
    p.add_argument("--cards-only", action="store_true",
                   help="Show only the per-card breakdown")

    # consolidate
    p = sub.add_parser("consolidate",
                       help="Create a merged-* ticket from a consolidation input file and mark source tickets as deduped")
    p.add_argument("--input", required=True,
                   help="Path to consolidation markdown input file")
    p.add_argument("--keep-input", action="store_true",
                   help="Do not delete the input file after successful consolidation")
    p.add_argument("--dry-run", action="store_true")

    # close-duplicate
    p = sub.add_parser("close-duplicate",
                       help="Close a single ticket as a duplicate of a bug tracked elsewhere")
    p.add_argument("--ticket", required=True, help="Ticket ID to close")
    p.add_argument("--duplicate-of", required=True,
                   help="Reference to the tracked bug (e.g. 'Bug BK' or 'tests/audit_bugs2.rs:528')")
    p.add_argument("--reason", help="Optional one-line reason")

    # dedup
    p = sub.add_parser("dedup",
                       help="Spawn a dedup agent on a seed set of tickets; the agent searches pipeline/tickets/ and may emit zero or more merged-* tickets")
    p.add_argument("--tickets", required=True,
                   help="Comma-separated seed ticket IDs (starting point; agent may include others or exclude these)")
    p.add_argument("--keep-input", action="store_true",
                   help="Do not delete the staging files after successful consolidation")
    add_common(p)

    args = parser.parse_args()

    TICKETS_DIR.mkdir(exist_ok=True)
    STAGING_DIR.mkdir(exist_ok=True)
    LOGS_DIR.mkdir(exist_ok=True)

    commands = {
        "audit": cmd_audit, "test": cmd_test, "fix": cmd_fix,
        "merge": cmd_merge, "abandon": cmd_abandon,
        "retry": cmd_retry,
        "tickets": cmd_tickets, "show": cmd_show,
        "status": cmd_status, "consolidate": cmd_consolidate,
        "close-duplicate": cmd_close_duplicate,
        "dedup": cmd_dedup, "report": cmd_report,
    }
    commands[args.command](args)


if __name__ == "__main__":
    main()
