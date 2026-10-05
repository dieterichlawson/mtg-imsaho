#!/usr/bin/env python3
"""Which kinds of question the fuzz campaign asked, and which it never did.

The nightly fuzz's oracle is the invariant checker, and an invariant says
nothing about a prompt the random seat never reaches: the seat went weeks
without ever answering an ordering prompt with ChosenOrder, reached
Ghoulcaller's Chant's second mode in 1.8% of casts, and drew a loyalty
ability once per target (#664-#667) — all found by hand, none red. So
every game run by scripts/fuzz.sh writes `--decision-stats`, a JSON count
of the prompt kinds it was asked, and this script sums them and compares
the names against the variants the engine defines, read from the source
the way mtg-player/tests/gui_protocol.rs reads the page's.

A kind the campaign never reached is a finding (the nightly workflow files
it), not an error here: the exit code is 0 whenever the stats could be read.

Usage: scripts/fuzz_reach.py --stats <dir-of-json> [--out <markdown>] [--repo <root>]
"""
import argparse
import glob
import json
import os
import re
import sys


def enum_variants(path: str, enum_name: str) -> list[str]:
    """The variant names of `pub enum <enum_name>` in a Rust source file."""
    src = open(path, encoding="utf-8").read()
    m = re.search(r"pub enum %s\b[^{]*\{" % re.escape(enum_name), src)
    if not m:
        sys.exit(f"{path}: no `pub enum {enum_name}`")
    depth, i, body = 1, m.end(), []
    while i < len(src) and depth:
        c = src[i]
        depth += (c == "{") - (c == "}")
        body.append(c)
        i += 1
    names = []
    for line in "".join(body).splitlines():
        # A variant is a capitalised identifier at four spaces of indent;
        # fields sit deeper and attributes/comments start otherwise.
        v = re.match(r"^    ([A-Z][A-Za-z0-9]*)\s*[{(,]?", line)
        if v:
            names.append(v.group(1))
    return names


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--stats", required=True, help="directory of --decision-stats JSON files")
    ap.add_argument("--out", help="write the markdown report here as well as to stdout")
    ap.add_argument("--repo", default=os.path.join(os.path.dirname(__file__), ".."))
    args = ap.parse_args()

    files = sorted(glob.glob(os.path.join(args.stats, "*.json")))
    counts: dict[str, int] = {}
    games, decisions, unreadable = 0, 0, 0
    widest: tuple[int, str] = (0, "")
    for f in files:
        try:
            d = json.load(open(f, encoding="utf-8"))
        except (OSError, ValueError):
            unreadable += 1
            continue
        games += 1
        decisions += int(d.get("decisions", 0))
        width = int(d.get("max_permanents", 0))
        if width > widest[0]:
            widest = (width, os.path.basename(f)[: -len(".json")])
        for k, n in (d.get("kinds") or {}).items():
            counts[k] = counts.get(k, 0) + int(n)

    expected = {"priority", "priority:pass-only", "set_prompt"}
    expected |= {"resolution:" + v for v in enum_variants(
        os.path.join(args.repo, "mtg-engine/src/state.rs"), "ResolutionChoiceKind")}
    expected |= {"combat:" + v for v in enum_variants(
        os.path.join(args.repo, "mtg-engine/src/actions.rs"), "CombatPrompt")}

    never = sorted(expected - set(counts))
    unknown = sorted(set(counts) - expected)

    lines = [f"### Prompt-kind reach: {games} games, {decisions} decisions"
             + (f", {unreadable} stats files unreadable" if unreadable else ""), ""]
    if games == 0:
        lines.append("no stats files found — the campaign wrote nothing to compare")
    else:
        lines += ["| kind | decisions | per game |", "|---|---:|---:|"]
        for k, n in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0])):
            lines.append(f"| `{k}` | {n} | {n / games:.2f} |")
        lines.append("")
        if widest[0]:
            lines.append(f"Widest board: {widest[0]} permanents (`{widest[1]}`).")
            lines.append("")
        if never:
            lines.append("**Never reached** (defined by the engine, asked in no game):")
            lines += [f"- `{k}`" for k in never]
        else:
            lines.append("Every kind the engine defines was reached at least once.")
        if unknown:
            lines.append("")
            lines.append("Counted but not in the engine's enums (the parser above missed them?):")
            lines += [f"- `{k}`" for k in unknown]
    report = "\n".join(lines) + "\n"
    sys.stdout.write(report)
    # One greppable line per gap, for the workflow's filing step.
    for k in never:
        print(f"NEVER REACHED: {k}")
    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            fh.write(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
