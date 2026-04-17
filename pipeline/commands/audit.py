"""Audit cards for bugs.

Spawns the auditor agent on each card (optionally in parallel), parses
its JSON findings, and creates one ticket per finding.
"""

from __future__ import annotations

import re
import threading
from dataclasses import dataclass

from pipeline import utils
from pipeline.agent import build_prompt, run_agent_loop, single_file_loader
from pipeline.metrics import log_finding, log_run
from pipeline.models import AuditReport, Finding, Ticket
from pipeline.state import Status
from pipeline.utils import now_iso, run_in_parallel, today


@dataclass
class AuditOneResult:
    """Summary of auditing one card: tickets created, timings, token usage."""

    card: str
    tickets_created: int
    duration: int
    tokens: int
    error: str = ""


# Concurrent appends to prompts/auditor-insights.md from parallel workers.
_INSIGHTS_LOCK = threading.Lock()


def cmd_audit(args):
    """Entry point for `./pipeline/cli.py audit`."""
    cards = _parse_cards(args.cards)
    print(f"\n{'=' * 60}\nAUDIT — {len(cards)} card(s)\n{'=' * 60}")
    for c in cards:
        print(f"  {c}")

    oracles = _fetch_oracles(cards)

    def run_one(card):
        return audit_one(card, oracles[card], args)

    results = run_in_parallel(run_one, list(oracles), args.parallelism)
    _print_summary(results)


def audit_one(card: str, oracle_text: str, args) -> AuditOneResult:
    """Audit a single card: spawn the agent, parse findings, write tickets."""
    snake = _card_to_snake(card)
    run_id = f"{today()}-{snake}-audit"
    staging_file = utils.STAGING_DIR / f"{run_id}.json"
    print(f"  [{card}] Spawning agent...")

    builder = build_prompt(
        "auditor",
        card=card,
        card_snake=snake,
        oracle=oracle_text,
        run_id=run_id,
    )
    parsed, result = run_agent_loop(
        build_prompt=builder,
        cwd=utils.PROJECT_ROOT,
        load_result=single_file_loader(staging_file, AuditReport.load),
        model=args.model,
        effort=args.effort,
        log_prefix=run_id,
        progress_prefix=f"  [{card}] ",
    )

    if result.get("is_error"):
        return _record_error(card, run_id, result, args.model)

    created = [
        _create_ticket(card, snake, finding, run_id, args.model, result)
        for finding in parsed.findings
    ]
    _append_insights(parsed.insights)
    if staging_file.exists():
        staging_file.unlink()

    log_run(
        "auditor",
        run_id=run_id,
        model=args.model,
        card=card,
        result=result,
        findings_created=len(created),
    )
    summary = "PASS" if parsed.is_pass else f"{len(created)} ticket(s)"
    print(
        f"  [{card}] Done: {summary} "
        f"({result['duration']}s, {result['tokens']} tok)"
    )
    return AuditOneResult(
        card=card,
        tickets_created=len(created),
        duration=result["duration"],
        tokens=result["tokens"],
    )


def _record_error(
    card: str, run_id: str, result: dict, model: str
) -> AuditOneResult:
    err = result["error_message"]
    print(
        f"  [{card}] AGENT ERROR: {err} "
        f"({result['duration']}s, {result['tokens']} tok)"
    )
    log_run(
        "auditor",
        run_id=run_id,
        model=model,
        card=card,
        result=result,
        validation_passed=False,
        rejection_reason=f"agent error: {err}",
        notes="agent_error",
    )
    return AuditOneResult(
        card=card,
        tickets_created=0,
        duration=result["duration"],
        tokens=result["tokens"],
        error=err,
    )


def _create_ticket(
    card: str,
    snake: str,
    finding: Finding,
    run_id: str,
    model: str,
    result: dict,
) -> str:
    new_id = Ticket.allocate_id(snake)
    num = int(new_id.rsplit("-", 1)[1])
    body = _render_audit_body(finding, snake, num)
    extra = {
        "card_file": f"mtg-engine/src/cards/isd/{snake}.rs",
        "created": now_iso(),
        "audit_run_id": run_id,
        "audit_model": model,
        "audit_tokens": str(result["tokens"]),
        "audit_duration": str(result["duration"]),
    }
    Ticket.create(new_id, status=Status.NEW, card=card, body=body, extra=extra)
    log_finding(
        new_id,
        "created",
        card=card,
        run_id=run_id,
        engine_file=",".join(finding.engine_path),
        description=finding.description[:80],
    )
    return new_id


def _append_insights(insights) -> None:
    if not insights:
        return
    with (
        _INSIGHTS_LOCK,
        open(utils.PROMPTS_DIR / "auditor-insights.md", "a") as f,
    ):
        for ins in insights:
            f.write(f"\n### {ins.title}\n{ins.description}\n")


def _parse_cards(raw: str) -> list[str]:
    sep = ";" if ";" in raw else ","
    return [c.strip() for c in raw.split(sep) if c.strip()]


def _fetch_oracles(cards: list[str]) -> dict[str, str]:
    """Fetch oracle text for every card, raising if any are missing."""
    print("\nFetching oracle texts...")
    out = {c: utils.get_oracle_text(c) for c in cards}
    missing = [c for c, text in out.items() if not text]
    if missing:
        raise RuntimeError(
            f"no oracle text for {len(missing)} card(s): {missing}. "
            f"Run `scripts/oracle_lookup.py add-card NAME` first."
        )
    return out


def _print_summary(results: list[AuditOneResult]) -> None:
    print(f"\n{'=' * 60}\nAUDIT SUMMARY\n{'=' * 60}")
    total = 0
    errors: list[AuditOneResult] = []
    for r in sorted(results, key=lambda x: x.card):
        total += r.tickets_created
        if r.error:
            status = "ERROR"
            errors.append(r)
        elif r.tickets_created == 0:
            status = "PASS"
        else:
            status = f"{r.tickets_created} ticket(s)"
        print(f"  {r.card:<30} {status:<15} {r.duration}s  {r.tokens} tok")
    print(f"\n  Total tickets created: {total}")
    if errors:
        print(f"\n  {len(errors)} agent error(s):")
        for r in errors:
            print(f"    {r.card}: {r.error}")


def _render_audit_body(finding: Finding, snake: str, ticket_num: int) -> str:
    parts = [
        "## Audit Finding",
        "",
        f"**Oracle text:**\n> {finding.oracle_quote}",
        "",
        f"**Code:**\n> {finding.code_quote}",
        "",
        f"**Description:**\n{finding.description}",
        "",
    ]
    if finding.engine_path:
        parts += (
            ["**Engine path:**"]
            + [f"- {p}" for p in finding.engine_path]
            + [""]
        )
    if finding.check:
        parts += [f"**Required check:** {finding.check}", ""]
    if finding.affected_cards:
        parts += (
            ["**Affected cards:**"]
            + [f"- {c}" for c in finding.affected_cards]
            + [""]
        )
    parts += ["## Tests", ""]
    if finding.tests:
        tests = [(t.slug, t.scenario) for t in finding.tests]
    else:
        default_scenario = (
            finding.description.split(".")[0][:240] or "See description above."
        )
        tests = [(f"test_{snake}_{ticket_num:02d}", default_scenario)]
    for slug, scenario in tests:
        parts += [
            f"### {slug}",
            "Source ticket: (new)",
            "Implementation: (not yet written)",
            f"Scenario: {scenario}",
            "",
        ]
    return "\n".join(parts).rstrip() + "\n"


def _card_to_snake(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")
