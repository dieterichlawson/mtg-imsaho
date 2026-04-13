#!/usr/bin/env python3
"""Pipeline metrics dashboard.

Usage:
    python3 pipeline/scripts/metrics.py              # Full dashboard
    python3 pipeline/scripts/metrics.py --funnel     # Just the funnel
    python3 pipeline/scripts/metrics.py --agents     # Agent performance
    python3 pipeline/scripts/metrics.py --trends     # Trends over time
    python3 pipeline/scripts/metrics.py --cards      # Per-card breakdown
    python3 pipeline/scripts/metrics.py --json       # Machine-readable output
"""

import argparse
import json
import sys
from collections import Counter, defaultdict
from datetime import datetime
from pathlib import Path

METRICS_DIR = Path(__file__).resolve().parent.parent / "metrics"
RUNS_FILE = METRICS_DIR / "runs.jsonl"
FINDINGS_FILE = METRICS_DIR / "findings.jsonl"
QUEUE_DIR = Path(__file__).resolve().parent.parent / "queue"


def load_jsonl(path: Path) -> list[dict]:
    if not path.exists():
        return []
    entries = []
    for line in path.read_text().strip().split("\n"):
        if line.strip():
            entries.append(json.loads(line))
    return entries


def count_queue_files() -> dict[str, int]:
    """Count files in each queue directory."""
    counts = {}
    for subdir in ["audit-findings",
                   "test-results/confirmed", "test-results/rejected",
                   "test-results/blocked",
                   "fix-results/ready-for-review", "fix-results/failed"]:
        d = QUEUE_DIR / subdir
        if d.exists():
            counts[subdir] = len([f for f in d.glob("*.md") if f.is_file()])
        else:
            counts[subdir] = 0
    return counts


def print_funnel(findings: list[dict], queue_counts: dict):
    """Print the finding lifecycle funnel."""
    events = Counter(f["event"] for f in findings)

    created = events.get("created", 0)
    confirmed = events.get("test_confirmed", 0)
    rejected = events.get("test_rejected", 0)
    blocked = events.get("test_blocked", 0)
    tested = confirmed + rejected + blocked
    fix_ok = events.get("fix_succeeded", 0)
    fix_fail = events.get("fix_failed", 0)
    fixed = fix_ok + fix_fail
    merged = events.get("merged", 0)

    print("## Funnel\n")
    print(f"  Findings created:     {created}")
    print(f"  Tests attempted:      {tested}")
    if tested > 0:
        print(f"    Confirmed (fails):  {confirmed}  ({confirmed*100//tested}%)")
        print(f"    Rejected (FP):      {rejected}  ({rejected*100//tested}%)")
        print(f"    Blocked:            {blocked}  ({blocked*100//tested}%)")
    print(f"  Fixes attempted:      {fixed}")
    if fixed > 0:
        print(f"    Succeeded:          {fix_ok}  ({fix_ok*100//fixed}%)")
        print(f"    Failed:             {fix_fail}  ({fix_fail*100//fixed}%)")
    print(f"  Merged:               {merged}")
    print()

    # Current queue state
    print("## Queue State\n")
    for subdir, count in sorted(queue_counts.items()):
        label = subdir.replace("/", " > ")
        print(f"  {label}: {count}")
    print()


def print_agent_perf(runs: list[dict]):
    """Print per-model, per-role performance."""
    print("## Agent Performance\n")

    # Group by (model, role)
    by_model_role = defaultdict(list)
    for r in runs:
        key = (r.get("model", "unknown"), r["role"])
        by_model_role[key].append(r)

    # Print table
    print(f"  {'Model':<25} {'Role':<14} {'Runs':>5} {'Validated':>10} {'Key Metric':>20}")
    print(f"  {'-'*25} {'-'*14} {'-'*5} {'-'*10} {'-'*20}")

    for (model, role), runs_list in sorted(by_model_role.items()):
        n = len(runs_list)
        validated = sum(1 for r in runs_list if r.get("validation_passed", True))

        # Key metric depends on role
        if role == "auditor":
            total_findings = sum(r.get("findings_created", 0) for r in runs_list)
            metric = f"{total_findings} findings"
        elif role == "test-writer":
            confirmed = sum(1 for r in runs_list if r.get("test_result") == "confirmed")
            rejected = sum(1 for r in runs_list if r.get("test_result") == "rejected")
            if confirmed + rejected > 0:
                fp_rate = rejected * 100 // (confirmed + rejected)
                metric = f"{fp_rate}% FP rate"
            else:
                metric = "no tests"
        elif role == "fixer":
            fixed = sum(1 for r in runs_list if r.get("fix_result") == "fixed")
            metric = f"{fixed}/{n} fixed"
        else:
            metric = ""

        print(f"  {model:<25} {role:<14} {n:>5} {validated:>10} {metric:>20}")
    print()


def print_trends(findings: list[dict]):
    """Print findings over time."""
    print("## Trends (by date)\n")

    # Group events by date
    by_date = defaultdict(lambda: Counter())
    for f in findings:
        date = f.get("timestamp", "")[:10]
        if date:
            by_date[date][f["event"]] += 1

    if not by_date:
        print("  No data yet.\n")
        return

    print(f"  {'Date':<12} {'Created':>8} {'Confirmed':>10} {'Rejected':>9} {'Fixed':>6} {'Merged':>7}")
    print(f"  {'-'*12} {'-'*8} {'-'*10} {'-'*9} {'-'*6} {'-'*7}")

    for date in sorted(by_date.keys()):
        c = by_date[date]
        print(f"  {date:<12} {c['created']:>8} {c['test_confirmed']:>10} "
              f"{c['test_rejected']:>9} {c['fix_succeeded']:>6} {c['merged']:>7}")
    print()


def print_cards(findings: list[dict]):
    """Print per-card breakdown."""
    print("## Cards\n")

    # Latest event per finding
    latest = {}
    for f in findings:
        fid = f["finding_id"]
        latest[fid] = f

    # Group by card
    by_card = defaultdict(list)
    for fid, f in latest.items():
        by_card[f.get("card", "unknown")].append(f)

    # Count states per card
    card_stats = []
    for card, card_findings in sorted(by_card.items()):
        events = Counter(f["event"] for f in card_findings)
        card_stats.append((card, len(card_findings), events))

    # Sort by finding count descending
    card_stats.sort(key=lambda x: -x[1])

    print(f"  {'Card':<30} {'Total':>6} {'Confirmed':>10} {'Rejected':>9} {'Fixed':>6}")
    print(f"  {'-'*30} {'-'*6} {'-'*10} {'-'*9} {'-'*6}")

    for card, total, events in card_stats[:30]:
        confirmed = events.get("test_confirmed", 0) + events.get("fix_succeeded", 0) + events.get("merged", 0)
        rejected = events.get("test_rejected", 0)
        fixed = events.get("fix_succeeded", 0) + events.get("merged", 0)
        print(f"  {card:<30} {total:>6} {confirmed:>10} {rejected:>9} {fixed:>6}")

    total_cards = len(by_card)
    clean_cards = sum(1 for c, fl in by_card.items()
                      if all(f["event"] in ("test_rejected",) for f in fl))
    print(f"\n  Total cards with findings: {total_cards}")
    print(f"  Cards where all findings rejected (clean): {clean_cards}")
    print()


def print_hotspots(findings: list[dict]):
    """Print engine file hotspots."""
    print("## Engine Hotspots\n")

    file_counts = Counter()
    for f in findings:
        if f["event"] == "created" and f.get("engine_file"):
            file_counts[f["engine_file"]] += 1

    if not file_counts:
        print("  No data yet.\n")
        return

    print(f"  {'Engine File':<50} {'Findings':>9}")
    print(f"  {'-'*50} {'-'*9}")
    for fpath, count in file_counts.most_common(15):
        print(f"  {fpath:<50} {count:>9}")
    print()


def print_efficiency(runs: list[dict]):
    """Print cost/efficiency metrics."""
    print("## Efficiency\n")

    by_role = defaultdict(list)
    for r in runs:
        by_role[r["role"]].append(r)

    print(f"  {'Role':<14} {'Runs':>5} {'Avg Tokens':>11} {'Avg Tools':>10} {'Avg Duration':>13}")
    print(f"  {'-'*14} {'-'*5} {'-'*11} {'-'*10} {'-'*13}")

    for role in ["auditor", "test-writer", "fixer", "log-miner"]:
        role_runs = by_role.get(role, [])
        if not role_runs:
            continue
        n = len(role_runs)
        tokens = [r.get("total_tokens", 0) for r in role_runs if r.get("total_tokens")]
        tools = [r.get("tool_uses", 0) for r in role_runs if r.get("tool_uses")]
        durations = [r.get("duration_seconds", 0) for r in role_runs if r.get("duration_seconds")]

        avg_tok = sum(tokens) // len(tokens) if tokens else 0
        avg_tools = sum(tools) // len(tools) if tools else 0
        avg_dur = sum(durations) // len(durations) if durations else 0
        dur_str = f"{avg_dur//60}m{avg_dur%60:02d}s" if avg_dur else "n/a"

        print(f"  {role:<14} {n:>5} {avg_tok:>11,} {avg_tools:>10} {dur_str:>13}")
    print()


def print_json(runs: list[dict], findings: list[dict], queue_counts: dict):
    """Print machine-readable summary."""
    events = Counter(f["event"] for f in findings)
    summary = {
        "timestamp": datetime.now().isoformat(),
        "funnel": {
            "created": events.get("created", 0),
            "test_confirmed": events.get("test_confirmed", 0),
            "test_rejected": events.get("test_rejected", 0),
            "test_blocked": events.get("test_blocked", 0),
            "fix_succeeded": events.get("fix_succeeded", 0),
            "fix_failed": events.get("fix_failed", 0),
            "merged": events.get("merged", 0),
        },
        "queue": queue_counts,
        "total_runs": len(runs),
        "runs_by_role": dict(Counter(r["role"] for r in runs)),
        "models_used": list(set(r.get("model", "unknown") for r in runs)),
    }
    print(json.dumps(summary, indent=2))


def main():
    parser = argparse.ArgumentParser(description="Pipeline metrics dashboard")
    parser.add_argument("--funnel", action="store_true", help="Show funnel only")
    parser.add_argument("--agents", action="store_true", help="Show agent performance")
    parser.add_argument("--trends", action="store_true", help="Show trends over time")
    parser.add_argument("--cards", action="store_true", help="Show per-card breakdown")
    parser.add_argument("--json", action="store_true", help="Machine-readable output")
    args = parser.parse_args()

    runs = load_jsonl(RUNS_FILE)
    findings = load_jsonl(FINDINGS_FILE)
    queue_counts = count_queue_files()

    if args.json:
        print_json(runs, findings, queue_counts)
        return

    show_all = not any([args.funnel, args.agents, args.trends, args.cards])

    print("=" * 60)
    print(f"PIPELINE METRICS — {datetime.now().strftime('%Y-%m-%d %H:%M')}")
    print("=" * 60)
    print()

    if show_all or args.funnel:
        print_funnel(findings, queue_counts)

    if show_all or args.agents:
        print_agent_perf(runs)

    if show_all or args.trends:
        print_trends(findings)

    if show_all or args.cards:
        print_cards(findings)

    if show_all:
        print_hotspots(findings)
        print_efficiency(runs)


if __name__ == "__main__":
    main()
