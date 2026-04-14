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
CARDS_DIR = PROJECT_ROOT / "mtg-engine" / "src" / "cards" / "isd"
ORACLE_SCRIPT = PROJECT_ROOT / "scripts" / "oracle_lookup.py"

DEFAULT_MODEL = "opus"

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


def merge_worktree(ticket_id: str) -> bool:
    """Merge a ticket's worktree branch into HEAD. Returns success."""
    branch = get_worktree_branch(ticket_id)
    result = subprocess.run(
        ["git", "merge", branch, "--no-edit"],
        capture_output=True, text=True, cwd=str(PROJECT_ROOT),
    )
    return result.returncode == 0


def run_agent_in(prompt: str, cwd: Path, model: str = DEFAULT_MODEL,
                 effort: str = DEFAULT_EFFORT) -> dict:
    """Run a claude agent in a specific directory. Returns usage stats."""
    cmd = [
        "claude", "-p", prompt,
        "--model", model,
        "--effort", effort,
        "--output-format", "json",
        "--permission-mode", "auto",
        "--no-session-persistence",
    ]
    start = time.time()
    result = subprocess.run(
        cmd, capture_output=True, text=True,
        cwd=str(cwd), timeout=900,
        env=subscription_env(),
    )
    elapsed = int(time.time() - start)
    tokens = 0
    tool_uses = 0
    is_error = False
    error_message = None
    try:
        data = json.loads(result.stdout)
        tokens = data.get("usage", {}).get("input_tokens", 0) + \
                 data.get("usage", {}).get("output_tokens", 0)
        tool_uses = data.get("num_turns", 0)
        if data.get("is_error"):
            is_error = True
            error_message = data.get("result") or "agent reported is_error=true"
    except (json.JSONDecodeError, KeyError, TypeError):
        pass
    if result.returncode != 0 and not is_error:
        is_error = True
        error_message = (result.stderr or result.stdout or "")[:200] or f"exit {result.returncode}"
    return {"returncode": result.returncode, "tokens": tokens,
            "tool_uses": tool_uses, "duration": elapsed,
            "is_error": is_error, "error_message": error_message}


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
              effort: str = DEFAULT_EFFORT) -> dict:
    """Run a claude agent via CLI. Returns usage stats.

    Agents get full tool access. Write restrictions enforced by post-validation.
    """
    cmd = [
        "claude", "-p", prompt,
        "--model", model,
        "--effort", effort,
        "--output-format", "json",
        "--permission-mode", "auto",
        "--no-session-persistence",
    ]

    start = time.time()
    result = subprocess.run(
        cmd, capture_output=True, text=True,
        cwd=str(PROJECT_ROOT),
        timeout=900,
        env=subscription_env(),
    )
    elapsed = int(time.time() - start)

    tokens = 0
    tool_uses = 0
    is_error = False
    error_message = None
    try:
        data = json.loads(result.stdout)
        tokens = data.get("usage", {}).get("input_tokens", 0) + \
                 data.get("usage", {}).get("output_tokens", 0)
        tool_uses = data.get("num_turns", 0)
        if data.get("is_error"):
            is_error = True
            error_message = data.get("result") or "agent reported is_error=true"
    except (json.JSONDecodeError, KeyError, TypeError):
        pass

    if result.returncode != 0 and not is_error:
        is_error = True
        error_message = (result.stderr or result.stdout or "")[:200] or f"exit {result.returncode}"

    return {
        "returncode": result.returncode,
        "tokens": tokens,
        "tool_uses": tool_uses,
        "duration": elapsed,
        "is_error": is_error,
        "error_message": error_message,
    }


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

        affected_match = re.search(r"\*\*Affected cards:\*\*\n(.+?)(?=\n##|$)", block, re.DOTALL)
        if affected_match:
            finding["affected_cards"] = affected_match.group(1).strip()

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
    """Parse test writer output from staging."""
    text = staging_path.read_text()

    result = {}
    for field in ["Status", "Test File", "Test Name", "Assertion Message", "Explanation", "Blocked By"]:
        match = re.search(rf"## {field}\n(.+?)(?=\n## |\Z)", text, re.DOTALL)
        if match:
            result[field.lower().replace(" ", "_")] = match.group(1).strip()

    return result


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
        result = run_agent(shared_prompt + "\n\n---\n\n" + per_agent,
                          args.model, args.effort)

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
                    body += f"**Affected cards:**\n{finding['affected_cards']}\n"

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

        per_agent = f"""## Ticket to test

{ticket["body"]}

### Oracle text (pre-fetched from Scryfall)

{oracle}

### Test file
Write your test to: `mtg-engine/tests/pipeline_bugs_{tid_snake}.rs`

### Staging output
Write your result to: `pipeline/staging/{tid}-test.md`
Use the format: ## Status, ## Test File, ## Test Name, ## Assertion Message, ## Explanation

### Ticket ID: {tid}
"""
        print(f"  [{tid}] Spawning agent in worktree...")
        result = run_agent_in(shared_prompt + "\n\n---\n\n" + per_agent,
                             wt_dir, args.model, args.effort)

        # Parse staging
        test_result = "rejected"
        test_name = ""
        test_file = ""
        validated = False

        if staging_file.exists():
            parsed = parse_test_staging(staging_file)
            test_result = parsed.get("status", "rejected")
            test_name = parsed.get("test_name", "")
            test_file = parsed.get("test_file", f"mtg-engine/tests/pipeline_bugs_{tid_snake}.rs")

            # Validate in the worktree
            if test_result == "confirmed" and test_name:
                test_path = wt_dir / test_file
                if test_path.exists():
                    val = subprocess.run(
                        [str(SCRIPTS_DIR / "validate_test.sh"), str(test_path), test_name],
                        capture_output=True, text=True, cwd=str(wt_dir),
                    )
                    validated = val.returncode == 0
                    if not validated:
                        test_result = "rejected"
                        print(f"  [{tid}] Validation FAILED")
                else:
                    test_result = "rejected"
            elif test_result in ("rejected", "blocked"):
                validated = True

            staging_file.unlink()

        # If rejected/blocked, remove the worktree
        if test_result in ("rejected", "blocked"):
            remove_worktree(tid)

        # Build test result section
        section = f"## Test Result\n\n"
        section += f"status: {test_result}\n"
        if test_name:
            section += f"test_name: {test_name}\n"
        if 'parsed' in dir() and parsed.get("assertion_message"):
            section += f"assertion: {parsed['assertion_message']}\n"
        if 'parsed' in dir() and parsed.get("explanation"):
            section += f"\n{parsed['explanation']}\n"
        if 'parsed' in dir() and parsed.get("blocked_by"):
            section += f"\nBlocked by: {parsed['blocked_by']}\n"

        append_ticket_section(tid, section)

        # Update ticket status + metadata
        extra_fm = {
            f"{test_result}_at": now_iso(),
            "test_run_id": f"{today()}-{tid}-test",
            "test_model": args.model,
            "test_tokens": str(result["tokens"]),
            "test_duration": str(result["duration"]),
        }
        if test_file:
            extra_fm["test_file"] = test_file
        if test_name:
            extra_fm["test_name"] = test_name
        if test_result == "confirmed":
            extra_fm["worktree"] = str(get_worktree_dir(tid))
        update_ticket_status(tid, test_result, extra_fm)

        # Log
        append_jsonl(METRICS_DIR / "runs.jsonl", {
            "run_id": f"{today()}-{tid}-test", "timestamp": now_iso(),
            "role": "test-writer", "model": args.model,
            "card": card, "finding_id": tid,
            "findings_created": 0, "test_result": test_result,
            "fix_result": None, "validation_passed": validated,
            "rejection_reason": None if validated else "validation failed",
            "total_tokens": result["tokens"], "tool_uses": result["tool_uses"],
            "duration_seconds": result["duration"], "notes": "",
        })
        append_jsonl(METRICS_DIR / "findings.jsonl", {
            "finding_id": tid, "timestamp": now_iso(),
            "event": f"test_{test_result}",
            "card": card, "source": "code-audit",
            "engine_file": "", "description": tid,
            "run_id": f"{today()}-{tid}-test",
            "test_name": test_name, "test_file": test_file,
        })

        print(f"  [{tid}] Done: {test_result} ({result['duration']}s, {result['tokens']} tok)")
        return {"ticket": tid, "result": test_result}

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
        print(f"  {r['ticket']:<30} {r['result']}")
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
    test_name = fm.get("test_name", "")

    # Reuse the ticket's worktree (created during test phase)
    wt_dir = get_worktree_dir(tid)
    if not wt_dir.exists():
        print(f"  No worktree found for {tid}. Run `test` first.")
        sys.exit(1)

    wt_staging = wt_dir / "pipeline" / "staging"
    wt_staging.mkdir(parents=True, exist_ok=True)
    staging_file = wt_staging / f"{tid}-fix.md"

    per_agent = f"""## Ticket to fix

{ticket["body"]}

### Failing test
- File: `{fm.get('test_file', '')}`
- Test name: `{test_name}`

### Staging output
Write your result to: `pipeline/staging/{tid}-fix.md`
Use the format: ## Status, ## Files Changed, ## Description

### Rules
- Only modify files under `mtg-engine/src/`
- Do NOT modify test files
- All tests must pass after your fix
- Zero compiler warnings
"""
    print(f"  Spawning agent in worktree {wt_dir.name}...")
    result = run_agent_in(shared_prompt + "\n\n---\n\n" + per_agent,
                         wt_dir, args.model, args.effort)

    fix_result = "failed"
    validated = False

    if staging_file.exists():
        parsed = parse_fix_staging(staging_file)
        fix_result = parsed.get("status", "failed")

        # Validate in the worktree
        if fix_result == "fixed" and test_name:
            val = subprocess.run(
                [str(SCRIPTS_DIR / "validate_fix.sh"), test_name],
                capture_output=True, text=True, cwd=str(wt_dir),
            )
            validated = val.returncode == 0
            if not validated:
                fix_result = "failed"
                print("  Validation FAILED")

        staging_file.unlink()

    # If failed, remove worktree so test can be re-run fresh
    if fix_result == "failed":
        remove_worktree(tid)
        print(f"  Removed worktree (fix failed)")

    # Build ticket section
    section = f"## Fix Result\n\n"
    section += f"status: {fix_result}\n"
    if 'parsed' in dir() and parsed.get("files_changed"):
        section += f"files_changed: {parsed['files_changed']}\n"
    if 'parsed' in dir() and parsed.get("description"):
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

    for status in ["new", "confirmed", "blocked", "fixed", "failed", "rejected", "merged"]:
        group = by_status.get(status, [])
        if not group:
            continue
        print(f"\n{status.upper()} ({len(group)})")
        for t in group:
            fm = t["frontmatter"]
            card = fm.get("card", "?")
            tid = fm.get("id", "?")
            print(f"  {tid:<30} {card}")


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

        # Clean up worktree
        remove_worktree(tid)

        # Update ticket
        update_ticket_status(tid, "merged", {"merged_at": now_iso()})
        append_jsonl(METRICS_DIR / "findings.jsonl", {
            "finding_id": tid, "timestamp": now_iso(),
            "event": "merged", "card": fm.get("card", ""),
            "source": "code-audit", "engine_file": "",
            "description": tid, "run_id": "merge",
        })
        print(f"  [{tid}] Merged and cleaned up")


# ─── Abandon command ──────────────────────────────────────────────

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

    # status
    sub.add_parser("status", help="Show metrics dashboard")

    args = parser.parse_args()

    TICKETS_DIR.mkdir(exist_ok=True)
    STAGING_DIR.mkdir(exist_ok=True)

    commands = {
        "audit": cmd_audit, "test": cmd_test, "fix": cmd_fix,
        "merge": cmd_merge, "abandon": cmd_abandon,
        "tickets": cmd_tickets, "show": cmd_show,
        "status": cmd_status,
    }
    commands[args.command](args)


if __name__ == "__main__":
    main()
