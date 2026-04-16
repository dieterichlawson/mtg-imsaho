#!/usr/bin/env python3
"""Pipeline CLI — find, test, fix, merge, and dedup bug tickets."""
from __future__ import annotations

import argparse
import concurrent.futures
import json
import re
import subprocess
import sys
import threading
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

# Allow `./pipeline/cli.py` to run as a script (not via `python -m`).
_THIS = Path(__file__).resolve()
if str(_THIS.parents[1]) not in sys.path:
    sys.path.insert(0, str(_THIS.parents[1]))

from pipeline._agent import (
    DEFAULT_MODEL, DEFAULT_EFFORT, MAX_ATTEMPTS,
    build_prompt as _build_prompt,
    run_agent_in as _run_agent_in, run_agent_loop)
from pipeline._staging import (
    StagingError, TEST_CONFIRMED, TEST_REJECTED, TEST_BLOCKED,
    VALID_TEST_STATUSES,
    load_audit as load_audit_staging,
    load_test as load_test_staging,
    load_fix as load_fix_staging,
    load_consolidation as load_consolidation_staging)
# ─── Paths ─────────────────────────────────────────────────────────
PROJECT_ROOT  = Path(__file__).resolve().parent.parent
PIPELINE_DIR  = PROJECT_ROOT / "pipeline"
TICKETS_DIR   = PIPELINE_DIR / "tickets"
ARCHIVE_DIR   = TICKETS_DIR / "archive"
STAGING_DIR   = PIPELINE_DIR / "staging"
PROMPTS_DIR   = PIPELINE_DIR / "prompts"
SCRIPTS_DIR   = PIPELINE_DIR / "scripts"
METRICS_DIR   = PIPELINE_DIR / "metrics"
LOGS_DIR      = PIPELINE_DIR / "logs"
WORKTREES_DIR = PROJECT_ROOT / ".worktrees"
ORACLE_SCRIPT = PROJECT_ROOT / "scripts" / "oracle_lookup.py"
# ─── Statuses ──────────────────────────────────────────────────────
STATUS_NEW            = "new"
STATUS_TESTED         = "tested"
STATUS_FIXED          = "fixed"
STATUS_SHIPPED        = "shipped"
STATUS_FIX_FAILED     = "fix_failed"
STATUS_FALSE_POSITIVE = "false_positive"
STATUS_CLOSED         = "closed"
OPEN_STATUSES     = {STATUS_NEW, STATUS_TESTED, STATUS_FIXED, STATUS_FIX_FAILED}
TERMINAL_STATUSES = {STATUS_SHIPPED, STATUS_FALSE_POSITIVE, STATUS_CLOSED}
CLOSED_REASON_ABSORBED  = "absorbed"
CLOSED_REASON_ABANDONED = "abandoned"
ABSORBABLE_STATUSES = {STATUS_NEW, STATUS_TESTED}

# Synchronizes concurrent appends to prompts/auditor-insights.md.
_INSIGHTS_LOCK = threading.Lock()
# ─── Small utilities ───────────────────────────────────────────────
def now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

def today() -> str:
    return datetime.now().strftime("%Y-%m-%d")

def card_to_snake(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")

def append_jsonl(path: Path, entry: dict) -> None:
    with open(path, "a") as f:
        f.write(json.dumps(entry) + "\n")
# ─── Worktrees ─────────────────────────────────────────────────────
def get_worktree_dir(ticket_id: str) -> Path:
    return WORKTREES_DIR / f"fix-{ticket_id}"

def get_worktree_branch(ticket_id: str) -> str:
    return f"fix/{ticket_id}"

def ensure_worktree(ticket_id: str) -> Path:
    wt = get_worktree_dir(ticket_id)
    if wt.exists():
        return wt
    WORKTREES_DIR.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "worktree", "add", "-b",
                    get_worktree_branch(ticket_id), str(wt), "HEAD"],
                   capture_output=True, check=True, cwd=str(PROJECT_ROOT))
    return wt

def remove_worktree(ticket_id: str) -> None:
    wt = get_worktree_dir(ticket_id)
    if wt.exists():
        subprocess.run(["git", "worktree", "remove", "--force", str(wt)],
                       capture_output=True, cwd=str(PROJECT_ROOT))
    subprocess.run(["git", "branch", "-D", get_worktree_branch(ticket_id)],
                   capture_output=True, cwd=str(PROJECT_ROOT))

def _branch_head_sha(branch: str) -> str:
    r = subprocess.run(["git", "rev-parse", branch],
                       capture_output=True, text=True, cwd=str(PROJECT_ROOT))
    return r.stdout.strip() if r.returncode == 0 else ""

def _reset_branch_to(tid: str, sha: str) -> None:
    wt = get_worktree_dir(tid)
    branch = get_worktree_branch(tid)
    if not wt.exists():
        WORKTREES_DIR.mkdir(parents=True, exist_ok=True)
        has = subprocess.run(["git", "rev-parse", "--verify", branch],
                             capture_output=True, cwd=str(PROJECT_ROOT)).returncode == 0
        cmd = (["git", "worktree", "add", str(wt), branch] if has
               else ["git", "worktree", "add", "-b", branch, str(wt), sha])
        subprocess.run(cmd, check=True, cwd=str(PROJECT_ROOT))
    subprocess.run(["git", "reset", "--hard", sha], check=True, cwd=str(wt))

def remove_logs_for_ticket(ticket_id: str) -> int:
    if not LOGS_DIR.exists():
        return 0
    # Match `<date>-<ticket_id>-*` — substring match would clobber logs
    # for differently-named tickets that contain our id.
    pat = re.compile(rf"\b{re.escape(ticket_id)}(?:[-.]|$)")
    removed = 0
    for f in LOGS_DIR.iterdir():
        if pat.search(f.name):
            try:
                f.unlink(); removed += 1
            except OSError:
                pass
    return removed
# ─── Ticket I/O ────────────────────────────────────────────────────
def parse_ticket(path: Path) -> dict:
    text = path.read_text()
    if not text.startswith("---"):
        return {"frontmatter": {}, "body": text, "path": path}
    end = text.index("---", 3)
    fm: dict[str, str] = {}
    for line in text[3:end].strip().split("\n"):
        if ":" in line:
            k, v = line.split(":", 1)
            fm[k.strip()] = v.strip()
    return {"frontmatter": fm, "body": text[end+3:].strip(), "path": path}

def ticket_path(tid: str) -> Path:
    active = TICKETS_DIR / f"{tid}.md"
    if active.exists():
        return active
    archived = ARCHIVE_DIR / f"{tid}.md"
    return archived if archived.exists() else active

def all_ticket_paths() -> list[Path]:
    active = list(TICKETS_DIR.glob("*.md"))
    archive = list(ARCHIVE_DIR.glob("*.md")) if ARCHIVE_DIR.exists() else []
    return sorted(active + archive, key=lambda p: p.stem)

def write_ticket(tid: str, frontmatter: dict, body: str) -> Path:
    path = ticket_path(tid)
    if not path.exists():
        path = TICKETS_DIR / f"{tid}.md"
    head = ["---"] + [f"{k}: {v}" for k, v in frontmatter.items()] + ["---"]
    path.write_text("\n".join(head) + "\n\n" + body + "\n")
    return path

def archive_ticket(tid: str) -> None:
    src = TICKETS_DIR / f"{tid}.md"
    if not src.exists():
        return
    ARCHIVE_DIR.mkdir(parents=True, exist_ok=True)
    src.rename(ARCHIVE_DIR / f"{tid}.md")

def unarchive_ticket(tid: str) -> None:
    src = ARCHIVE_DIR / f"{tid}.md"
    if src.exists():
        src.rename(TICKETS_DIR / f"{tid}.md")

def transition(tid: str, status: str,
               set_: dict | None = None,
               unset: tuple[str, ...] = ()) -> None:
    """Canonical status change. Writes frontmatter, archives or unarchives
    based on terminality. Every status transition should flow through this."""
    t = parse_ticket(ticket_path(tid))
    fm = t["frontmatter"]
    fm["status"] = status
    if set_:
        fm.update(set_)
    for k in unset:
        fm.pop(k, None)
    write_ticket(tid, fm, t["body"])
    if status in TERMINAL_STATUSES:
        archive_ticket(tid)
    else:
        unarchive_ticket(tid)

def list_tickets(status: str | None = None, card: str | None = None) -> list[dict]:
    out = []
    for f in all_ticket_paths():
        t = parse_ticket(f)
        fm = t["frontmatter"]
        if status and fm.get("status") != status:
            continue
        if card and card.lower() not in fm.get("card", "").lower():
            continue
        out.append(t)
    return out

def append_ticket_section(tid: str, section: str) -> None:
    t = parse_ticket(ticket_path(tid))
    write_ticket(tid, t["frontmatter"], t["body"] + "\n\n" + section)

# ─── Tests section ─────────────────────────────────────────────────
def _parse_tests_section(body: str) -> list[dict]:
    m = re.search(r"##\s+Tests\n(.*?)(?=\n##\s|\Z)", body, re.DOTALL)
    if not m:
        return []
    entries = []
    for block in re.split(r"\n(?=###\s+)", m.group(1)):
        block = block.strip()
        if not block.startswith("###"):
            continue
        head, _, rest = block.partition("\n")
        src = re.search(r"Source ticket:\s*(.+)", rest)
        impl = re.search(r"Implementation:\s*(.+)", rest)
        entries.append({
            "slug": head[3:].strip(),
            "source_ticket": src.group(1).strip() if src else None,
            "implementation": impl.group(1).strip() if impl else ""})
    return entries

def update_tests_section_impls(tid: str, impls: dict[str, str]) -> None:
    t = parse_ticket(ticket_path(tid))
    current: str | None = None
    out: list[str] = []
    for line in t["body"].split("\n"):
        if line.startswith("### "):
            current = line[4:].strip()
            out.append(line)
        elif current and current in impls and line.startswith("Implementation:"):
            out.append(f"Implementation: {impls[current]}")
        else:
            out.append(line)
    write_ticket(tid, t["frontmatter"], "\n".join(out).rstrip() + "\n")
# ─── Agent runner (shim to _agent module) ─────────────────────────
_AGENT_SETTINGS = PIPELINE_DIR / "agent-settings.json"

def run_agent_in(prompt: str, cwd: Path,
                 model: str = DEFAULT_MODEL, effort: str = DEFAULT_EFFORT,
                 log_path: Path | None = None,
                 progress_prefix: str = "") -> dict:
    return _run_agent_in(prompt, cwd, model, effort,
                         log_path=log_path, progress_prefix=progress_prefix,
                         settings_path=_AGENT_SETTINGS)

def run_agent(prompt: str, model: str = DEFAULT_MODEL,
              effort: str = DEFAULT_EFFORT, log_path: Path | None = None,
              progress_prefix: str = "") -> dict:
    return run_agent_in(prompt, PROJECT_ROOT, model, effort,
                        log_path=log_path, progress_prefix=progress_prefix)

def build_prompt(role: str, **ctx):
    return _build_prompt(role, PROMPTS_DIR, **ctx)

def get_oracle_text(card_name: str) -> str | None:
    for verb in ("lookup", "fetch"):
        r = subprocess.run(["python3", str(ORACLE_SCRIPT), verb, card_name],
                           capture_output=True, text=True, cwd=str(PROJECT_ROOT))
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
    return None
# ─── Metrics helpers ───────────────────────────────────────────────
def log_run(role: str, *, run_id: str, model: str, card: str,
            result: dict, validation_passed: bool = True,
            finding_id: str | None = None, findings_created: int = 0,
            test_result: str | None = None, fix_result: str | None = None,
            rejection_reason: str | None = None, notes: str = "") -> None:
    append_jsonl(METRICS_DIR / "runs.jsonl", {
        "run_id": run_id, "timestamp": now_iso(), "role": role,
        "model": model, "card": card, "finding_id": finding_id,
        "findings_created": findings_created,
        "test_result": test_result, "fix_result": fix_result,
        "validation_passed": validation_passed,
        "rejection_reason": rejection_reason,
        "total_tokens": result.get("tokens", 0),
        "tool_uses": result.get("tool_uses", 0),
        "duration_seconds": result.get("duration", 0), "notes": notes})

def log_finding(finding_id: str, event: str, *,
                card: str = "", run_id: str = "", **extra) -> None:
    append_jsonl(METRICS_DIR / "findings.jsonl", {
        "finding_id": finding_id, "timestamp": now_iso(), "event": event,
        "card": card, "source": "code-audit", "engine_file": "",
        "description": finding_id, "run_id": run_id, **extra})

def _loop_kwargs(log_prefix: str, progress_prefix: str, args) -> dict:
    # Pass `spawn=run_agent_in` so tests that patch `cli.run_agent_in` are
    # exercised even though `run_agent_loop` lives in `_agent`.
    return {"logs_dir": LOGS_DIR, "log_prefix": log_prefix,
            "progress_prefix": progress_prefix,
            "model": args.model, "effort": args.effort,
            "settings_path": _AGENT_SETTINGS,
            "spawn": run_agent_in}
# ─── Audit ─────────────────────────────────────────────────────────
def cmd_audit(args):
    sep = ";" if ";" in args.cards else ","
    cards = [c.strip() for c in args.cards.split(sep) if c.strip()]
    print(f"\n{'='*60}\nAUDIT — {len(cards)} card(s)\n{'='*60}")
    for c in cards:
        print(f"  {c}")
    if args.dry_run:
        print("\n(dry run)"); return

    print("\nFetching oracle texts...")
    oracles: dict[str, str] = {}
    for c in cards:
        o = get_oracle_text(c)
        if o:
            oracles[c] = o
        else:
            print(f"  SKIP: no oracle text for {c}")
    if not oracles:
        print("No cards to audit."); return

    def audit_one(card: str) -> dict:
        snake = card_to_snake(card)
        run_id = f"{today()}-{snake}-audit"
        staging_file = STAGING_DIR / f"{run_id}.json"
        print(f"  [{card}] Spawning agent...")
        builder = build_prompt("auditor", card=card, card_snake=snake,
                               oracle=oracles[card], run_id=run_id)
        parsed, result = run_agent_loop(
            build_prompt=builder, cwd=PROJECT_ROOT,
            staging_file=staging_file, loader=load_audit_staging,
            **_loop_kwargs(run_id, f"  [{card}] ", args))

        if result.get("is_error") or parsed is None:
            err = result.get("error_message") or "no valid staging"
            print(f"  [{card}] AGENT ERROR: {err} "
                  f"({result['duration']}s, {result['tokens']} tok)")
            log_run("auditor", run_id=run_id, model=args.model, card=card,
                    result=result, validation_passed=False,
                    rejection_reason=f"agent error: {err}", notes="agent_error")
            return {"card": card, "tickets": 0,
                    "duration": result["duration"],
                    "tokens": result["tokens"], "error": err}

        nums = [int(m.group(1)) for p in all_ticket_paths()
                if (m := re.match(rf"{snake}-(\d+)$", p.stem))]
        next_num = max(nums, default=0) + 1
        created: list[str] = []
        for finding in parsed["findings"]:
            tid = f"{snake}-{next_num:02d}"
            next_num += 1
            body = _render_audit_body(finding, snake, next_num - 1)
            fm = {"id": tid, "status": STATUS_NEW, "card": card,
                  "card_file": f"mtg-engine/src/cards/isd/{snake}.rs",
                  "created": now_iso(), "audit_run_id": run_id,
                  "audit_model": args.model,
                  "audit_tokens": result["tokens"],
                  "audit_duration": result["duration"]}
            write_ticket(tid, fm, body)
            created.append(tid)
            log_finding(tid, "created", card=card, run_id=run_id,
                        engine_file=finding.get("engine_path", ""),
                        description=finding.get("description", "")[:80])

        if parsed["insights"]:
            with _INSIGHTS_LOCK, open(PROMPTS_DIR / "auditor-insights.md", "a") as f:
                for ins in parsed["insights"]:
                    f.write(f"\n### {ins['title']}\n{ins['description']}\n")
        if staging_file.exists():
            staging_file.unlink()
        log_run("auditor", run_id=run_id, model=args.model, card=card,
                result=result, findings_created=len(created))
        st = "PASS" if parsed["is_pass"] else f"{len(created)} ticket(s)"
        print(f"  [{card}] Done: {st} "
              f"({result['duration']}s, {result['tokens']} tok)")
        return {"card": card, "tickets": len(created),
                "duration": result["duration"], "tokens": result["tokens"]}

    to_run = list(oracles)
    if args.parallelism > 1:
        print(f"\nRunning {len(to_run)} audits (parallelism={args.parallelism})...")
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallelism) as pool:
            fs = {pool.submit(audit_one, c): c for c in to_run}
            results = [f.result() for f in concurrent.futures.as_completed(fs)]
    else:
        results = [audit_one(c) for c in to_run]

    print(f"\n{'='*60}\nAUDIT SUMMARY\n{'='*60}")
    total = 0; errors = []
    for r in sorted(results, key=lambda x: x["card"]):
        t = r["tickets"]; total += t
        if r.get("error"):
            status = "ERROR"; errors.append(r)
        else:
            status = "PASS" if t == 0 else f"{t} ticket(s)"
        print(f"  {r['card']:<30} {status:<15} {r['duration']}s  {r['tokens']} tok")
    print(f"\n  Total tickets created: {total}")
    if errors:
        print(f"\n  {len(errors)} agent error(s):")
        for r in errors:
            print(f"    {r['card']}: {r['error']}")

def _render_audit_body(finding: dict, snake: str, ticket_num: int) -> str:
    parts = ["## Audit Finding", "",
             f"**Oracle text:**\n> {finding['oracle_quote']}", "",
             f"**Code:**\n> {finding['code_quote']}", "",
             f"**Description:**\n{finding['description']}", ""]
    if finding["engine_path"]:
        parts += ["**Engine path:**"] + [f"- {p}" for p in finding["engine_path"]] + [""]
    if finding["check"]:
        parts += [f"**Required check:** {finding['check']}", ""]
    if finding["affected_cards"]:
        parts += ["**Affected cards:**"] + [f"- {c}" for c in finding["affected_cards"]] + [""]
    parts += ["## Tests", ""]
    tests = finding["tests"] or [{
        "slug": f"test_{snake}_{ticket_num:02d}",
        "scenario": (finding.get("description", "").split(".")[0][:240]
                     or "See description above.")}]
    for t in tests:
        parts += [f"### {t['slug']}", "Source ticket: (new)",
                  "Implementation: (not yet written)",
                  f"Scenario: {t['scenario']}", ""]
    return "\n".join(parts).rstrip() + "\n"
# ─── Test ──────────────────────────────────────────────────────────
def cmd_test(args):
    if args.tickets:
        tickets = [parse_ticket(ticket_path(t.strip()))
                   for t in args.tickets.split(",")]
    else:
        tickets = list_tickets(status=STATUS_NEW)
    tickets = [t for t in tickets if t["frontmatter"].get("status") == STATUS_NEW]
    if not tickets:
        print("No tickets to test."); return

    print(f"\n{'='*60}\nTEST WRITER — {len(tickets)} ticket(s)\n{'='*60}")
    for t in tickets:
        fm = t["frontmatter"]
        print(f"  {fm.get('id', '?')}: {fm.get('card', '?')}")
    if args.dry_run:
        print("\n(dry run)"); return

    if args.parallelism > 1:
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallelism) as pool:
            fs = {pool.submit(_test_one, t, args): t for t in tickets}
            results = [f.result() for f in concurrent.futures.as_completed(fs)]
    else:
        results = [_test_one(t, args) for t in tickets]

    print(f"\n{'='*60}\nTEST SUMMARY\n{'='*60}")
    for r in sorted(results, key=lambda x: x["ticket"]):
        detail = f"{r['confirmed']}/{r['total']} tests" if r['total'] else ""
        print(f"  {r['ticket']:<40} {r['result']:<10} {detail}")
    c  = sum(1 for r in results if r["result"] == "confirmed")
    rj = sum(1 for r in results if r["result"] == "rejected")
    bl = sum(1 for r in results if r["result"] == "blocked")
    print(f"\n  Confirmed: {c}  Rejected: {rj}  Blocked: {bl}")

def _test_one(ticket: dict, args) -> dict:
    fm = ticket["frontmatter"]
    tid = fm["id"]
    card = fm.get("card", "unknown")
    tid_snake = tid.replace("-", "_")
    oracle = get_oracle_text(card) or "Oracle text not available"
    wt = ensure_worktree(tid)
    (wt / "pipeline" / "staging").mkdir(parents=True, exist_ok=True)
    staging_file = wt / "pipeline" / "staging" / f"{tid}-test.json"
    default_file = f"mtg-engine/tests/pipeline_bugs_{tid_snake}.rs"

    engine_note = ""
    if fm.get("allow_engine_edits") == "true":
        engine_note = (
            "\n### Engine edits allowed this run\n"
            "A prior attempt reported the test cannot be expressed without "
            "an engine change. For this run only, you may modify "
            "`mtg-engine/src/**` to add the minimal surface area needed. "
            "See `## Engine Change Needed` in the ticket body.\n")

    builder = build_prompt("test-writer",
        ticket_body=ticket["body"], oracle=oracle, engine_note=engine_note,
        tid_snake=tid_snake, tid=tid)

    st: dict[str, Any] = {"aggregate": "false_positive", "per_test": [],
                          "impls": {}, "test_file": default_file}
    expected = {e["slug"] for e in _parse_tests_section(ticket["body"])}

    def validate(parsed, _result, attempt):
        if not parsed["tests"]:
            return ("Your staging JSON had no entries under `tests`. "
                    "Emit one per slug in the ticket's `## Tests` section.")
        st["test_file"] = parsed["test_file"] or default_file
        per_test = list(parsed["tests"])
        st["per_test"] = per_test

        errs = []
        for t in per_test:
            if t["status"] == TEST_CONFIRMED and not t["test_name"]:
                errs.append(f"- `{t['slug']}`: confirmed but test_name empty")
            if t["status"] == TEST_BLOCKED and not (t["blocked_by"] or "").strip():
                errs.append(f"- `{t['slug']}`: blocked but blocked_by empty")
        if errs:
            return ("Your staging JSON is missing required fields:\n"
                    + "\n".join(errs)
                    + "\nConfirmed MUST have `test_name`. "
                      "Blocked MUST have `blocked_by`.")

        impls: dict[str, str] = {}
        for t in per_test:
            slug = t["slug"]
            if t["status"] == TEST_CONFIRMED:
                p = wt / st["test_file"]
                if not p.exists():
                    t["status"] = TEST_REJECTED
                    impls[slug] = "rejected: test file missing"
                    continue
                val = subprocess.run(
                    [str(SCRIPTS_DIR / "validate_test.sh"), str(p), t["test_name"]],
                    capture_output=True, text=True, cwd=str(wt))
                if val.returncode == 0:
                    impls[slug] = f"{st['test_file']}::{t['test_name']}"
                else:
                    t["status"] = TEST_REJECTED
                    impls[slug] = "rejected: validation failed"
            elif t["status"] == TEST_REJECTED:
                impls[slug] = f"rejected: {t.get('explanation','')[:80]}"
            elif t["status"] == TEST_BLOCKED:
                impls[slug] = "(not yet written)"
        st["impls"] = impls

        statuses = {t["status"] for t in per_test}
        if TEST_BLOCKED in statuses:
            st["aggregate"] = "needs_engine"; return None
        if TEST_CONFIRMED not in statuses:
            st["aggregate"] = "false_positive"; return None

        missing = expected - {t["slug"] for t in per_test}
        if missing:
            st["aggregate"] = "false_positive"
            if attempt < MAX_ATTEMPTS:
                return (f"Ticket's `## Tests` section lists {len(expected)} "
                        f"slug(s) but staging has no confirmed entry for every "
                        f"one. Missing: {sorted(missing)}.")
            return None
        dirty = subprocess.run(["git", "status", "--porcelain"],
                               capture_output=True, text=True,
                               cwd=str(wt)).stdout.strip()
        if dirty:
            st["aggregate"] = "false_positive"
            if attempt < MAX_ATTEMPTS:
                return (f"Every per-test validation succeeded but worktree "
                        f"is not clean:\n\n{dirty}\n\nRun `git add -A && "
                        f"git commit` before writing staging output.")
            return None
        st["aggregate"] = "tested"
        return None

    print(f"  [{tid}] Spawning agent in worktree...")
    _, result = run_agent_loop(
        build_prompt=builder, cwd=wt, staging_file=staging_file,
        loader=load_test_staging, validator=validate,
        **_loop_kwargs(f"{today()}-{tid}-test", f"  [{tid}] ", args))

    aggregate = st["aggregate"]
    per_test = st["per_test"]
    validated = (aggregate == "tested")
    if st["impls"]:
        update_tests_section_impls(tid, st["impls"])
    if aggregate == "false_positive":
        remove_worktree(tid)

    lines = ["## Test Run Results", ""]
    for t in per_test:
        lines.append(f"- **{t['slug']}** — {t['status']}")
        if t.get("test_name"):
            lines.append(f"  - test fn: `{t['test_name']}`")
        if t.get("assertion_message"):
            lines.append(f"  - assertion: {t['assertion_message']}")
        if t.get("blocked_by"):
            lines.append(f"  - blocked by: {t['blocked_by']}")
    append_ticket_section(tid, "\n".join(lines))

    n_conf = sum(1 for t in per_test if t["status"] == TEST_CONFIRMED)
    extra = {"test_run_id": f"{today()}-{tid}-test",
             "test_model": args.model,
             "test_tokens": str(result["tokens"]),
             "test_duration": str(result["duration"]),
             "test_file": st["test_file"],
             "tests_confirmed": str(n_conf),
             "tests_total": str(len(per_test))}
    if aggregate == "tested":
        extra["tested_at"] = now_iso()
        extra["worktree"] = str(get_worktree_dir(tid))
        extra["tested_sha"] = _branch_head_sha(get_worktree_branch(tid))
        transition(tid, STATUS_TESTED, set_=extra)
    elif aggregate == "needs_engine":
        extra["allow_engine_edits"] = "true"
        extra["engine_block_at"] = now_iso()
        transition(tid, STATUS_NEW, set_=extra)
        blocked = [f"- **{t['slug']}**: {t['blocked_by']}" for t in per_test
                   if t["status"] == TEST_BLOCKED and t.get("blocked_by")]
        if blocked:
            append_ticket_section(tid,
                "## Engine Change Needed\n\n"
                "The test-writer could not express the following test(s) "
                "without engine changes. Retry will allow "
                "`mtg-engine/src/**` edits.\n\n" + "\n".join(blocked))
    else:
        extra["false_positive_at"] = now_iso()
        transition(tid, STATUS_FALSE_POSITIVE, set_=extra)

    log_run("test-writer", run_id=f"{today()}-{tid}-test",
            model=args.model, card=card, result=result,
            finding_id=tid, test_result=aggregate,
            validation_passed=validated,
            rejection_reason=None if validated else "one-or-more tests failed validation",
            notes=f"{n_conf}/{len(per_test)} tests confirmed")
    log_finding(tid, f"test_{aggregate}", card=card,
                run_id=f"{today()}-{tid}-test", test_file=st["test_file"])
    print(f"  [{tid}] Done: {aggregate} — {n_conf}/{len(per_test)} tests "
          f"({result['duration']}s, {result['tokens']} tok)")
    return {"ticket": tid, "result": aggregate,
            "confirmed": n_conf, "total": len(per_test)}
# ─── Fix ───────────────────────────────────────────────────────────
def cmd_fix(args):
    if args.ticket:
        tickets = [parse_ticket(ticket_path(args.ticket))]
    else:
        tickets = list_tickets(status=STATUS_TESTED)[:1]
    if not tickets:
        print(f"No {STATUS_TESTED} tickets to fix."); return

    ticket = tickets[0]
    fm = ticket["frontmatter"]
    tid = fm["id"]
    if fm.get("status") != STATUS_TESTED:
        print(f"Ticket {tid}: status={fm.get('status')}, not {STATUS_TESTED}. "
              f"Use `retry --to tested` to rewind.")
        sys.exit(1)
    card = fm.get("card", "unknown")
    print(f"\n{'='*60}\nFIXER — {tid} ({card})\n{'='*60}")
    if args.dry_run:
        print("\n(dry run)"); return

    wt = get_worktree_dir(tid)
    if not wt.exists():
        print(f"  No worktree found for {tid}. Run `test` first.")
        sys.exit(1)
    (wt / "pipeline" / "staging").mkdir(parents=True, exist_ok=True)
    staging_file = wt / "pipeline" / "staging" / f"{tid}-fix.json"

    test_fns = [impl.split("::", 1)[1]
                for ct in _parse_tests_section(ticket["body"])
                if "::" in (impl := ct.get("implementation", "").strip())]
    failing = "\n".join(f"- `{n}`" for n in test_fns) \
              or "- (see ticket `## Tests` section)"
    builder = build_prompt("fixer",
        ticket_body=ticket["body"], num_tests=len(test_fns),
        test_file=fm.get("test_file", ""),
        failing_tests_block=failing, tid=tid)

    def validate(parsed, _result, attempt):
        if parsed["status"] != "fixed":
            if not parsed["description"] and attempt < MAX_ATTEMPTS:
                return ("Previous attempt reported status=failed but omitted "
                        "`description`. A post-mortem is the only artifact "
                        "of a failed run — explain what you tried and what "
                        "engine-level change would be required.")
            return None
        val = subprocess.run([str(SCRIPTS_DIR / "validate_fix.sh")],
                             capture_output=True, text=True, cwd=str(wt))
        if val.returncode == 0:
            return None
        tail = (val.stdout[-1500:] if val.stdout else "") + \
               (val.stderr[-500:] if val.stderr else "")
        print(f"  Validation FAILED (attempt {attempt}):\n{tail}")
        return (f"validate_fix.sh rejected your previous attempt. Output "
                f"(last 1500 chars):\n\n{tail}\n\nFix and retry. Remember "
                f"to commit changes before running validate_fix.sh.")

    print(f"  Spawning agent in worktree {wt.name}...")
    parsed, result = run_agent_loop(
        build_prompt=builder, cwd=wt, staging_file=staging_file,
        loader=load_fix_staging, validator=validate,
        **_loop_kwargs(f"{today()}-{tid}-fix", f"  [{tid}] ", args))

    parsed = parsed or {"status": "failed", "files_changed": [], "description": ""}
    fix_result = parsed["status"]
    validated = False
    if fix_result == "fixed":
        val = subprocess.run([str(SCRIPTS_DIR / "validate_fix.sh")],
                             capture_output=True, cwd=str(wt))
        validated = val.returncode == 0
        if not validated:
            fix_result = "failed"

    status = STATUS_FIXED if fix_result == "fixed" else STATUS_FIX_FAILED
    if status == STATUS_FIX_FAILED:
        print(f"  Worktree preserved for inspection: {wt}")
        print(f"  Run `./pipeline/cli.py retry {tid}` to rewind, or "
              f"`close {tid}` to give up.")

    sec = [f"## Fix Result\n\nstatus: {status}"]
    if parsed.get("files_changed"):
        sec.append("files_changed:")
        sec += [f"- {p}" for p in parsed["files_changed"]]
    if parsed.get("description"):
        sec += ["", parsed["description"]]
    append_ticket_section(tid, "\n".join(sec))

    extra = {f"{status}_at": now_iso(),
             "fix_run_id": f"{today()}-{tid}-fix",
             "fix_model": args.model,
             "fix_tokens": str(result["tokens"]),
             "fix_duration": str(result["duration"])}
    if status == STATUS_FIXED:
        extra["fixed_sha"] = _branch_head_sha(get_worktree_branch(tid))
    transition(tid, status, set_=extra)

    log_run("fixer", run_id=f"{today()}-{tid}-fix", model=args.model,
            card=card, result=result, finding_id=tid, fix_result=fix_result,
            validation_passed=validated,
            rejection_reason=None if validated else "validation failed")
    log_finding(tid, "fix_succeeded" if fix_result == "fixed" else "fix_failed",
                card=card, run_id=f"{today()}-{tid}-fix")
    print(f"\n  Result: {fix_result} "
          f"({result['duration']}s, {result['tokens']} tok)")
# ─── Merge ─────────────────────────────────────────────────────────
def cmd_merge(args):
    if args.ticket_id == "all":
        tickets = list_tickets(status=STATUS_FIXED)
    else:
        tickets = []
        for tid in [t.strip() for t in args.ticket_id.split(",")]:
            path = ticket_path(tid)
            if not path.exists():
                print(f"  Skipping {tid}: not found"); continue
            t = parse_ticket(path)
            if t["frontmatter"].get("status") != STATUS_FIXED:
                print(f"  Skipping {tid}: status is "
                      f"{t['frontmatter'].get('status')}, not {STATUS_FIXED}")
                continue
            tickets.append(t)
    if not tickets:
        print(f"No {STATUS_FIXED} tickets to merge."); return

    print(f"\n{'='*60}\nMERGE — {len(tickets)} ticket(s)\n{'='*60}")
    for t in tickets:
        print(f"  {t['frontmatter']['id']:<30} "
              f"{t['frontmatter'].get('card', '?')}")
    if args.dry_run:
        print("\n(dry run)"); return

    for ticket in tickets:
        fm = ticket["frontmatter"]
        tid = fm["id"]
        wt = get_worktree_dir(tid)
        print(f"\n  [{tid}] Merging...")
        if not wt.exists():
            print(f"  [{tid}] No worktree found — skipping"); continue
        r = subprocess.run(["git", "merge", get_worktree_branch(tid), "--no-edit"],
                           capture_output=True, text=True, cwd=str(PROJECT_ROOT))
        if r.returncode != 0:
            print(f"  [{tid}] Merge FAILED: {r.stderr[:200]}"); continue
        merge_sha = subprocess.run(["git", "rev-parse", "HEAD"],
            capture_output=True, text=True, cwd=str(PROJECT_ROOT)).stdout.strip()
        remove_worktree(tid)
        n = remove_logs_for_ticket(tid)
        if n:
            print(f"  [{tid}] Removed {n} agent log file(s)")
        extra = {"shipped_at": now_iso()}
        if merge_sha:
            extra["shipped_sha"] = merge_sha
        transition(tid, STATUS_SHIPPED, set_=extra, unset=("worktree",))
        log_finding(tid, "shipped", card=fm.get("card", ""), run_id="merge")
        print(f"  [{tid}] Shipped and cleaned up")
# ─── Retry ─────────────────────────────────────────────────────────
_FIX_KEYS = ("fix_run_id", "fix_model", "fix_tokens", "fix_duration",
             "fixed_at", "fixed_sha", "fix_failed_at")
_TEST_KEYS = ("test_run_id", "test_model", "test_tokens", "test_duration",
              "test_file", "tests_confirmed", "tests_total",
              "tested_at", "tested_sha", "worktree", "engine_block_at",
              "false_positive_at", "allow_engine_edits")

def _archive_attempt(tid: str) -> None:
    """Fold ## Fix Result / ## Test Run Results / ## Engine Change Needed
    into a numbered `## Attempt N` archive, preserving history."""
    t = parse_ticket(ticket_path(tid))
    body = t["body"]
    archivable = ("## Fix Result", "## Test Run Results", "## Engine Change Needed")
    nums = [int(m.group(1)) for m in re.finditer(
        r"^##\s+Attempt\s+(\d+)\b", body, re.MULTILINE)]
    next_n = (max(nums) + 1) if nums else 1

    sections: list[str] = []
    keep: list[str] = []
    current: list[str] | None = None
    for line in body.split("\n"):
        if line.startswith("## ") and any(line.startswith(h) for h in archivable):
            if current is not None:
                sections.append("\n".join(current).rstrip())
            current = [line]
        elif line.startswith("## ") and current is not None:
            sections.append("\n".join(current).rstrip())
            current = None
            keep.append(line)
        elif current is not None:
            current.append(line)
        else:
            keep.append(line)
    if current is not None:
        sections.append("\n".join(current).rstrip())
    if not sections:
        return
    new = "\n".join(keep).rstrip() + "\n\n"
    new += f"## Attempt {next_n}\n\n" + "\n\n".join(sections).rstrip() + "\n"
    write_ticket(tid, t["frontmatter"], new)

def _clear_impls(tid: str) -> None:
    t = parse_ticket(ticket_path(tid))
    body = re.sub(r"(?m)^Implementation:.*$",
                  "Implementation: (not yet written)", t["body"])
    write_ticket(tid, t["frontmatter"], body)

def cmd_retry(args):
    tid = args.ticket_id
    fm = parse_ticket(ticket_path(tid))["frontmatter"]
    status = fm.get("status", "")
    if status == STATUS_FALSE_POSITIVE and not args.force:
        print(f"ERROR: {tid} is {STATUS_FALSE_POSITIVE}; use --force to override")
        sys.exit(1)
    if status in (STATUS_SHIPPED, STATUS_CLOSED):
        print(f"ERROR: {tid} is terminal ({status}); cannot retry")
        sys.exit(1)

    target = args.to or (STATUS_TESTED if status == STATUS_FIX_FAILED else STATUS_NEW)
    if target not in (STATUS_NEW, STATUS_TESTED):
        print(f"ERROR: --to must be {STATUS_NEW} or {STATUS_TESTED}, got {target!r}")
        sys.exit(1)

    if target == STATUS_TESTED:
        sha = fm.get("tested_sha", "")
        if not sha:
            print(f"ERROR: {tid} has no tested_sha; cannot rewind to "
                  f"{STATUS_TESTED}. Use --to {STATUS_NEW} instead.")
            sys.exit(1)
    else:
        sha = "master"

    print(f"[{tid}] Retry: {status} → {target}  (reset to {sha})")
    _archive_attempt(tid)
    if target == STATUS_NEW:
        if get_worktree_dir(tid).exists():
            remove_worktree(tid)
        _clear_impls(tid)
        transition(tid, STATUS_NEW, unset=_FIX_KEYS + _TEST_KEYS)
    else:
        _reset_branch_to(tid, sha)
        transition(tid, STATUS_TESTED, unset=_FIX_KEYS)

    nxt = "test" if target == STATUS_NEW else "fix"
    flag = "--tickets" if target == STATUS_NEW else "--ticket"
    print(f"[{tid}] Ready. Run `./pipeline/cli.py {nxt} {flag} {tid}` to continue.")
# ─── Close / tickets / show / status / report ─────────────────────
def cmd_close(args):
    tid = args.ticket_id
    extra = {"closed_reason": CLOSED_REASON_ABANDONED, "closed_at": now_iso()}
    if args.note:
        extra["closed_note"] = args.note
    transition(tid, STATUS_CLOSED, set_=extra)
    if get_worktree_dir(tid).exists():
        remove_worktree(tid)
        print(f"Removed worktree for {tid}")
    n = remove_logs_for_ticket(tid)
    if n:
        print(f"Removed {n} agent log file(s) for {tid}")
    print(f"Closed {tid} (reason={CLOSED_REASON_ABANDONED})")

def cmd_tickets(args):
    tickets = list_tickets(status=args.status, card=args.card)
    if not tickets:
        print("No tickets found."); return
    by_status: dict[str, list] = {}
    for t in tickets:
        by_status.setdefault(t["frontmatter"].get("status", "unknown"), []).append(t)
    primary = [STATUS_NEW, STATUS_TESTED, STATUS_FIXED, STATUS_FIX_FAILED,
               STATUS_FALSE_POSITIVE, STATUS_SHIPPED, STATUS_CLOSED]
    for s in primary + sorted(s for s in by_status if s not in primary):
        group = by_status.get(s, [])
        if not group:
            continue
        print(f"\n{s.upper()} ({len(group)})")
        for t in group:
            fm = t["frontmatter"]
            target = fm.get("duplicate_of") or fm.get("deduped_into") \
                     or fm.get("absorbed_into")
            extra = f" → {target}" if target else ""
            print(f"  {fm.get('id', '?'):<30} {fm.get('card', '?')}{extra}")

def cmd_show(args):
    path = ticket_path(args.ticket_id)
    if not path.exists():
        print(f"Ticket not found: {args.ticket_id}"); sys.exit(1)
    print(path.read_text())

def cmd_status(args):
    subprocess.run(["python3", str(SCRIPTS_DIR / "metrics.py")],
                   cwd=str(PROJECT_ROOT))

def cmd_report(args):
    extra = []
    if args.audits_only:
        extra.append("--audits-only")
    if args.cards_only:
        extra.append("--cards-only")
    subprocess.run(["python3", str(SCRIPTS_DIR / "report.py"), *extra],
                   cwd=str(PROJECT_ROOT))
# ─── Consolidate + dedup ───────────────────────────────────────────
# Factory-built so they can access the patchable cli.run_agent,
# cli.parse_ticket, etc. at call time (needed by the journey tests).
from pipeline._dedup import make_commands as _make_dedup_commands
cmd_consolidate, cmd_dedup = _make_dedup_commands(
    cli_module=sys.modules[__name__])
# ─── Main / argparse ───────────────────────────────────────────────
def _add_common(p: argparse.ArgumentParser) -> None:
    p.add_argument("--model", default=DEFAULT_MODEL)
    p.add_argument("--effort", default=DEFAULT_EFFORT)
    p.add_argument("--dry-run", action="store_true")

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Pipeline CLI — manage bug-finding and fixing agents")
    sub = parser.add_subparsers(dest="command", required=True)

    def P(name, **kw): return sub.add_parser(name, **kw)

    p = P("audit", help="Audit cards for bugs"); _add_common(p)
    p.add_argument("--cards", required=True,
                   help="Names separated by ',' (or ';' if any name has a comma)")
    p.add_argument("--parallelism", type=int, default=1)

    p = P("test", help="Write tests for new tickets"); _add_common(p)
    p.add_argument("--tickets", help="Comma-separated ticket IDs")
    p.add_argument("--parallelism", type=int, default=1)

    p = P("fix", help="Fix a tested ticket"); _add_common(p)
    p.add_argument("--ticket")

    p = P("tickets", help="List tickets")
    p.add_argument("--status"); p.add_argument("--card")

    p = P("show", help="Display a ticket"); p.add_argument("ticket_id")

    p = P("merge", help="Merge fixed ticket(s) to HEAD"); _add_common(p)
    p.add_argument("ticket_id", help="Ticket ID, comma-separated IDs, or 'all'")

    p = P("retry", help="Rewind a ticket to an earlier phase")
    p.add_argument("ticket_id")
    p.add_argument("--to", choices=[STATUS_NEW, STATUS_TESTED], default=None)
    p.add_argument("--force", action="store_true",
                   help=f"Allow retry on {STATUS_FALSE_POSITIVE}")

    p = P("close", help=f"Close as {CLOSED_REASON_ABANDONED}")
    p.add_argument("ticket_id"); p.add_argument("--note")

    P("status", help="Show metrics dashboard")

    p = P("report", help="Coverage + backlog report")
    p.add_argument("--audits-only", action="store_true")
    p.add_argument("--cards-only", action="store_true")

    p = P("consolidate", help="Ingest a consolidation staging file → merged-*")
    p.add_argument("--input", required=True)
    p.add_argument("--keep-input", action="store_true")
    p.add_argument("--dry-run", action="store_true")

    p = P("dedup", help="Spawn a dedup agent on a seed set of tickets")
    p.add_argument("--tickets", required=True, help="Comma-separated seed ticket IDs")
    p.add_argument("--keep-input", action="store_true"); _add_common(p)

    args = parser.parse_args()
    TICKETS_DIR.mkdir(exist_ok=True)
    STAGING_DIR.mkdir(exist_ok=True)
    LOGS_DIR.mkdir(exist_ok=True)

    {"audit": cmd_audit, "test": cmd_test, "fix": cmd_fix,
     "merge": cmd_merge, "retry": cmd_retry, "close": cmd_close,
     "tickets": cmd_tickets, "show": cmd_show, "status": cmd_status,
     "consolidate": cmd_consolidate, "dedup": cmd_dedup, "report": cmd_report,
     }[args.command](args)

if __name__ == "__main__":
    main()
