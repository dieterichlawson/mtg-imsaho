#!/usr/bin/env python3
"""Replay kept positions as one decision each, under one seat configuration.

    scripts/decision-eval/replay.py --positions DIR --out DIR --name NAME \
        --p1 cc:claude-sonnet-4-6 [--env MTG_LLM_THINKING=off ...] \
        [--bin target/release/mtg-runner] [--jobs 4] [--samples 1] [--limit N]

Each position (a save `capture-claude.py` kept) is resumed with
`mtg-runner --resume <pos> --p1 <spec> --p2 random --stop-after 1`, which
asks the LLM seat the decision it was asked live — the same prompt, byte
for byte, when the binary renders prompts the way the one that played the
game did (`save_files.rs` pins that) — and stops. One JSON line per
position and sample goes to `<out>/<name>.jsonl`: the action chosen, the
tokens every call of the decision recorded (a cast and its target are two
calls), and the bytes of the system prompt and first prompt. A position
already in the file is skipped, so an interrupted run picks up where it
stopped. `compare.py` reads the files. A position the runner says it
resumed under another system prompt than the live one (another
MTG_LLM_HISTORY, MTG_CLAUDE_CODE_ANSWERS or MTG_LLM_PASS_UNTIL) carries
`unlike_live` in its row, and the run ends with a WARN counting them.

Every call is a real model call when `--p1` names a real seat: on the
`cc` seat that is plan quota, not an API bill. Never pass a `claude:` or
`gemini:` seat here without meaning to pay for it.
"""
import argparse
import concurrent.futures as cf
import json
import os
import re
import subprocess
import sys

import runstamp

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
# What mtg-runner --resume says of a position whose system prompt it cannot
# make as it was live; mtg-draft-replay says the same of a pick.
UNLIKE = ": its system prompt is not the live one: "


def prompt_bodies(log):
    bodies, cur = [], None
    for line in log.split("\n"):
        if re.match(r"^\d{4}-", line):
            if cur is not None:
                bodies.append("\n".join(cur))
                cur = None
            if "\tPROMPT [" in line:
                cur = []
        elif cur is not None:
            cur.append(line)
    if cur is not None:
        bodies.append("\n".join(cur))
    return bodies


def system_bytes(log):
    m = re.search(r"\tSYSTEM[^\n]*\n(.*?)\n\d{4}-", log, re.S)
    return len(m.group(1).encode()) if m else None


def usage(log):
    total = {"calls": 0, "in": 0, "out": 0, "thinking": 0, "cache_read": 0, "cache_write": 0}
    for m in re.finditer(r"\tUSAGE \[[^\]]*\]\t(.*)", log):
        total["calls"] += 1
        for k, v in re.findall(r"([a-z_]+) (\d+)", m.group(1)):
            if k in total:
                total[k] += int(v)
    return total


def run_one(args, position, sample, logdir):
    base = os.path.basename(position)[: -len(".save")]
    log = os.path.join(logdir, f"{base}.s{sample}.log")
    # The runner appends to an existing --log, so a rerun would read the
    # usage of both runs.
    if os.path.exists(log):
        os.remove(log)
    env = dict(os.environ)
    env.setdefault("STUB_CALLS", "/dev/null")
    for kv in args.env:
        k, _, v = kv.partition("=")
        env[k] = v
    cmd = [args.bin, "--resume", position, "--p1", args.p1, "--p2", "random", "--quiet",
           "--stop-after", "1", "--log", log]
    try:
        out = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True, timeout=args.timeout)
        stdout, rc = out.stdout, out.returncode
        # A position resumed under another system prompt than the live one
        # (#920's game twin): said with exit 0, and kept in the row.
        unlike = [l.split(UNLIKE, 1)[1] for l in out.stderr.splitlines() if UNLIKE in l]
    except subprocess.TimeoutExpired:
        stdout, rc, unlike = "", "timeout", []
    m = re.search(r"^DECISION 1 p(\d) (.*)$", stdout, re.M)
    text = open(log, encoding="utf-8", errors="replace").read() if os.path.exists(log) else ""
    prompts = prompt_bodies(text)
    return {
        "position": base, "sample": sample, "config": args.name,
        "seat": int(m.group(1)) if m else None,
        "action": json.loads(m.group(2)) if m else None,
        "exit": rc,
        "usage": usage(text),
        "system_bytes": system_bytes(text),
        "prompt_bytes": len(prompts[0].encode()) if prompts else None,
        "prompts": len(prompts),
        "fallback": bool(re.search(r"\t(MALFORMED|VALIDATION|BLOCKER_VALIDATION|API_ERROR)", text)),
        **({"unlike_live": unlike} if unlike else {}),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--positions", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--name", required=True)
    ap.add_argument("--p1", required=True)
    ap.add_argument("--env", action="append", default=[])
    ap.add_argument("--bin", default=os.path.join(ROOT, "target/release/mtg-runner"))
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--samples", type=int, default=1)
    ap.add_argument("--limit", type=int)
    ap.add_argument("--timeout", type=int, default=900)
    args = ap.parse_args()
    args.bin = os.path.abspath(args.bin)
    if args.p1.split(":")[0] in ("claude", "ai", "llm", "gemini"):
        sys.exit(f"replay.py: --p1 {args.p1} is a metered API seat; refusing without an explicit edit")
    positions = sorted(os.path.join(os.path.abspath(args.positions), p)
                       for p in os.listdir(args.positions) if p.endswith(".save"))
    if args.limit:
        positions = positions[: args.limit]
    os.makedirs(args.out, exist_ok=True)
    logdir = os.path.join(args.out, args.name)
    os.makedirs(logdir, exist_ok=True)
    result_path = os.path.join(args.out, f"{args.name}.jsonl")
    runstamp.check(args.out, args.name, {
        "positions": os.path.abspath(args.positions), "p1": args.p1, "env": sorted(args.env),
        "bin": os.path.abspath(args.bin), "cli": os.environ.get("CLAUDE_CODE_BIN", "claude"),
        "inherited": runstamp.inherited(),
    }, os.path.basename(__file__))
    done = set()
    if os.path.exists(result_path):
        for line in open(result_path):
            r = json.loads(line)
            if r.get("action") is not None:
                done.add((r["position"], r["sample"]))
    todo = [(p, s) for p in positions for s in range(args.samples)
            if (os.path.basename(p)[:-5], s) not in done]
    # Singular at one, as the runner's counts are (#914, #946's review).
    print(f"{args.name}: {len(todo)} {'decision' if len(todo) == 1 else 'decisions'} to replay ({len(done)} already done)", file=sys.stderr)
    with cf.ThreadPoolExecutor(args.jobs) as pool, open(result_path, "a") as out:
        futures = [pool.submit(run_one, args, p, s, logdir) for p, s in todo]
        unlike = {}
        for n, f in enumerate(cf.as_completed(futures), 1):
            r = f.result()
            for why in r.get("unlike_live", []):
                if why not in unlike:
                    print(f"{args.name}: WARN position {r['position']}'s system prompt is not the live one: {why}",
                          file=sys.stderr)
                unlike[why] = unlike.get(why, 0) + 1
            out.write(json.dumps(r) + "\n")
            out.flush()
            if n % 10 == 0 or n == len(futures):
                print(f"{args.name}: {n}/{len(futures)}", file=sys.stderr)
    for why, count in unlike.items():
        print(f"{args.name}: WARN {count} {'decision was' if count == 1 else 'decisions were'} asked under another system prompt than the live one: {why}",
              file=sys.stderr)


if __name__ == "__main__":
    main()
