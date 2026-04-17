"""Audit cards for bugs — spawn the auditor agent, create tickets for each finding."""
from __future__ import annotations

import re
import threading

from pipeline import oracle, paths, ticket
from pipeline.agent import build_prompt, run_agent_loop
from pipeline.metrics import log_finding, log_run, now_iso, today
from pipeline.parallel import run_in_parallel
from pipeline.staging import load_audit
from pipeline.state import Status

# Concurrent appends to prompts/auditor-insights.md from parallel workers.
_INSIGHTS_LOCK = threading.Lock()


def cmd_audit(args):
    cards = _parse_cards(args.cards)
    print(f"\n{'='*60}\nAUDIT — {len(cards)} card(s)\n{'='*60}")
    for c in cards:
        print(f"  {c}")
    if args.dry_run:
        print("\n(dry run)")
        return

    oracles = _fetch_oracles(cards)
    if not oracles:
        print("No cards to audit.")
        return

    def run_one(card):
        return audit_one(card, oracles[card], args)

    results = run_in_parallel(run_one, list(oracles), args.parallelism)
    _print_summary(results)


def audit_one(card: str, oracle_text: str, args) -> dict:
    """Audit a single card: spawn the agent, parse findings, write tickets."""
    snake = _card_to_snake(card)
    run_id = f"{today()}-{snake}-audit"
    staging_file = paths.STAGING_DIR / f"{run_id}.json"
    print(f"  [{card}] Spawning agent...")

    builder = build_prompt("auditor", card=card, card_snake=snake,
                           oracle=oracle_text, run_id=run_id)
    parsed, result = run_agent_loop(
        build_prompt=builder, cwd=paths.PROJECT_ROOT,
        staging_file=staging_file, loader=load_audit,
        model=args.model, effort=args.effort,
        log_prefix=run_id, progress_prefix=f"  [{card}] ")

    if result.get("is_error") or parsed is None:
        return _record_error(card, run_id, result, args.model)

    created = [_create_ticket(card, snake, finding, run_id, args.model, result)
               for finding in parsed["findings"]]
    _append_insights(parsed["insights"])
    if staging_file.exists():
        staging_file.unlink()

    log_run("auditor", run_id=run_id, model=args.model, card=card,
            result=result, findings_created=len(created))
    summary = "PASS" if parsed["is_pass"] else f"{len(created)} ticket(s)"
    print(f"  [{card}] Done: {summary} "
          f"({result['duration']}s, {result['tokens']} tok)")
    return {"card": card, "tickets": len(created),
            "duration": result["duration"], "tokens": result["tokens"]}


def _record_error(card, run_id, result, model):
    err = result.get("error_message") or "no valid staging"
    print(f"  [{card}] AGENT ERROR: {err} "
          f"({result['duration']}s, {result['tokens']} tok)")
    log_run("auditor", run_id=run_id, model=model, card=card,
            result=result, validation_passed=False,
            rejection_reason=f"agent error: {err}", notes="agent_error")
    return {"card": card, "tickets": 0, "duration": result["duration"],
            "tokens": result["tokens"], "error": err}


def _create_ticket(card, snake, finding, run_id, model, result):
    stem = snake
    new_id = ticket.allocate_id(stem)
    num = int(new_id.rsplit("-", 1)[1])
    body = _render_audit_body(finding, snake, num)
    extra = {"card_file": f"mtg-engine/src/cards/isd/{snake}.rs",
             "created": now_iso(), "audit_run_id": run_id,
             "audit_model": model,
             "audit_tokens": str(result["tokens"]),
             "audit_duration": str(result["duration"])}
    ticket.new(new_id, status=Status.NEW, card=card, body=body, extra=extra)
    log_finding(new_id, "created", card=card, run_id=run_id,
                engine_file=finding.get("engine_path", ""),
                description=finding.get("description", "")[:80])
    return new_id


def _append_insights(insights: list[dict]) -> None:
    if not insights:
        return
    with _INSIGHTS_LOCK, open(paths.PROMPTS_DIR / "auditor-insights.md", "a") as f:
        for ins in insights:
            f.write(f"\n### {ins['title']}\n{ins['description']}\n")


def _parse_cards(raw: str) -> list[str]:
    sep = ";" if ";" in raw else ","
    return [c.strip() for c in raw.split(sep) if c.strip()]


def _fetch_oracles(cards: list[str]) -> dict[str, str]:
    print("\nFetching oracle texts...")
    out: dict[str, str] = {}
    for c in cards:
        text = oracle.get_oracle_text(c)
        if text:
            out[c] = text
        else:
            print(f"  SKIP: no oracle text for {c}")
    return out


def _print_summary(results: list[dict]) -> None:
    print(f"\n{'='*60}\nAUDIT SUMMARY\n{'='*60}")
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
        print(f"  {r['card']:<30} {status:<15} "
              f"{r['duration']}s  {r['tokens']} tok")
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


def _card_to_snake(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")
