#!/usr/bin/env python3
"""Ask kept draft picks again, under one seat configuration.

    scripts/decision-eval/draft-replay.py --positions picks.jsonl --out DIR --name NAME \
        --model cc:claude-haiku-5-5 [--env MTG_LLM_THINKING=off ...] \
        [--bin target/release/mtg-draft-replay] [--jobs 4] [--samples 1] [--limit N]

`picks.jsonl` is what `mtg-draft-runner` keeps when `MTG_DRAFT_POSITIONS`
names a file: one line per pick of a real draft. Each position is asked
again as one pick by `mtg-draft-replay` — the same prompt, byte for byte,
when the binary renders prompts as the one that drafted did; another
build's own prompt for the same pack and pool otherwise. One JSON line per
position and sample goes to `<out>/<name>.jsonl`, in the rows
`compare.py` reads: the card chosen as the action, the tokens the pick
cost, and the bytes of the system prompt and the prompt. A position
already in the file is skipped, so an interrupted run picks up where it
stopped.

Every pick is a real model call: on a `cc` seat that is plan quota, not
an API bill. A metered seat is refused.
"""
import argparse
import concurrent.futures as cf
import json
import os
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def run_one(args, pid, position, sample):
    env = dict(os.environ)
    env.setdefault("STUB_CALLS", "/dev/null")
    for kv in args.env:
        k, _, v = kv.partition("=")
        env[k] = v
    with tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False) as f:
        f.write(position)
        path = f.name
    try:
        out = subprocess.run([args.bin, "--positions", path, "--model", args.model],
                             cwd=ROOT, env=env, capture_output=True, text=True, timeout=args.timeout)
        line = next((l for l in out.stdout.splitlines() if l.startswith("{")), None)
        rc = out.returncode
    except subprocess.TimeoutExpired:
        line, rc = None, "timeout"
    finally:
        os.remove(path)
    r = json.loads(line) if line else {}
    u = r.get("usage") or {}
    return {
        "position": pid, "sample": sample, "config": args.name, "exit": rc,
        "action": r.get("chosen"), "live": r.get("live"),
        "usage": {"calls": u.get("calls", 0), "in": u.get("input", 0), "out": u.get("output", 0),
                  "thinking": 0, "cache_read": u.get("cache_read", 0), "cache_write": u.get("cache_write", 0)},
        "system_bytes": r.get("system_bytes"), "prompt_bytes": r.get("prompt_bytes"),
        "fallback": bool(r.get("substituted")),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--positions", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--name", required=True)
    ap.add_argument("--model", required=True)
    ap.add_argument("--env", action="append", default=[])
    ap.add_argument("--bin", default=os.path.join(ROOT, "target/release/mtg-draft-replay"))
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--samples", type=int, default=1)
    ap.add_argument("--limit", type=int)
    ap.add_argument("--timeout", type=int, default=900)
    args = ap.parse_args()
    args.bin = os.path.abspath(args.bin)
    if args.model.split(":")[0] not in ("cc", "claude-code"):
        sys.exit(f"draft-replay.py: --model {args.model} is a metered API seat; refusing without an explicit edit")
    lines = [l for l in open(args.positions) if l.strip()]
    if args.limit:
        lines = lines[: args.limit]
    os.makedirs(args.out, exist_ok=True)
    result_path = os.path.join(args.out, f"{args.name}.jsonl")
    done = set()
    if os.path.exists(result_path):
        for line in open(result_path):
            r = json.loads(line)
            if r.get("action") is not None:
                done.add((r["position"], r["sample"]))
    todo = [(f"{i:04d}", l, s) for i, l in enumerate(lines) for s in range(args.samples)
            if (f"{i:04d}", s) not in done]
    print(f"{args.name}: {len(todo)} picks to replay ({len(done)} already done)", file=sys.stderr)
    with cf.ThreadPoolExecutor(args.jobs) as pool, open(result_path, "a") as out:
        futures = [pool.submit(run_one, args, pid, l, s) for pid, l, s in todo]
        for n, f in enumerate(cf.as_completed(futures), 1):
            out.write(json.dumps(f.result()) + "\n")
            out.flush()
            if n % 10 == 0 or n == len(futures):
                print(f"{args.name}: {n}/{len(futures)}", file=sys.stderr)


if __name__ == "__main__":
    main()
