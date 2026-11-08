#!/usr/bin/env python3
"""Ask a stronger model which of two picks is better, where a guide changed one.

    scripts/decision-eval/judge-picks.py --dir DIR --config NAME \
        [--ref none-a --ref2 none-b] [--model claude-sonnet-5-5] [--jobs 4]

`DIR` holds a night's `positions.jsonl` and `draft-replay.py` results. For
every *settled* pick (`--ref` and `--ref2` chose the same card: the
drafter agrees with itself without a guide) where `--config` chose another
card, the judge is shown the pack, the drafter's pool and the two cards as
A and B, in an order fixed by a hash of the position, and asked which is
the better pick. It never sees the guide or which card a guide chose, so
the yardstick is not the guide's own. A pick the runner substituted
(`"fallback": true`) is no answer: its position is left out and named. Rows go to `DIR/judge-<config>.jsonl`
(a pick already judged is skipped) and the tally is printed: how often the
guide's card was judged better, worse or even, with a 95% interval on the
share of decided picks that went the guide's way, and how often the judge
said A, as a check on order bias.

Every judgement is one `claude -p` call on the CLI's own login: plan
quota, never a metered key (ANTHROPIC_API_KEY is removed from its env).
"""
import argparse
import concurrent.futures as cf
import hashlib
import json
import math
import os
import re
import subprocess
import sys
import uuid

from answered import answered

SYSTEM = """You are an expert Magic: The Gathering limited player judging booster-draft picks in
Innistrad (2011, set code ISD): eight drafters, three packs of 14 cards, decks of 40, best-of-three matches.
You are shown one pick: the pack on offer, the cards the drafter has already taken, and two candidate
picks from that pack, A and B. Say which candidate gives this drafter the better chance of a winning deck,
given the pool so far and the stage of the draft. Answer with only a JSON object:
{"reason": "<one or two sentences>", "better": "A" | "B" | "even"}
Use "even" only when neither is clearly better."""


def claude(message, model, timeout=600):
    env = {k: v for k, v in os.environ.items()
           if k not in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "CLAUDECODE") and not k.startswith("CLAUDE_CODE_SESSION")}
    cmd = [os.environ.get("CLAUDE_CODE_BIN", "claude"), "-p", "--output-format", "json", "--tools", "",
           "--system-prompt", SYSTEM, "--session-id", str(uuid.uuid4()), "--model", model, "--effort", "medium"]
    for _ in range(3):
        r = subprocess.run(cmd, input=message, capture_output=True, text=True, env=env, cwd="/tmp", timeout=timeout)
        try:
            out = json.loads(r.stdout)
        except json.JSONDecodeError:
            out = None
        if out and not out.get("is_error"):
            return out.get("result", "")
    return None


def answers(path):
    """The seat's own answers by position, and the positions where the
    runner substituted a pick and no row answered it."""
    out, substituted = {}, set()
    for line in open(path):
        r = json.loads(line)
        if answered(r):
            out[int(r["position"])] = r["action"]
        elif r.get("fallback"):
            substituted.add(int(r["position"]))
    return out, substituted - set(out)


def prompt(p, a, b):
    pool = p["pool"]
    return (f"Pack {p['pack_number']}, pick {p['pick_index']}.\n\n"
            f"Already taken ({len(pool)}): {', '.join(pool) if pool else 'nothing yet'}\n\n"
            f"The pack: {', '.join(p['available'])}\n\n"
            f"A: {a}\nB: {b}\n\nWhich is the better pick?")


def judge(args, i, p, guide_card, ref_card):
    flip = int(hashlib.sha256(f"{args.config}:{i}".encode()).hexdigest(), 16) % 2
    a, b = (guide_card, ref_card) if flip else (ref_card, guide_card)
    text = claude(prompt(p, a, b), args.model) or ""
    m = re.search(r"\{.*\}", text, re.S)
    try:
        v = json.loads(m.group(0)) if m else {}
    except json.JSONDecodeError:
        v = {}
    said = v.get("better") if v.get("better") in ("A", "B", "even") else None
    verdict = None if said is None else ("even" if said == "even" else
                                         ("guide" if (said == "A") == bool(flip) else "ref"))
    return {"position": i, "pack": p["pack_number"], "pick": p["pick_index"], "a": a, "b": b,
            "guide_card": guide_card, "ref_card": ref_card, "said": said, "verdict": verdict,
            "reason": v.get("reason")}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dir", required=True)
    ap.add_argument("--config", required=True)
    ap.add_argument("--ref", default="none-a")
    ap.add_argument("--ref2", default="none-b")
    ap.add_argument("--model", default="claude-sonnet-5-5")
    ap.add_argument("--jobs", type=int, default=4)
    args = ap.parse_args()
    pos = [json.loads(l) for l in open(os.path.join(args.dir, "positions.jsonl")) if l.strip()]
    names = (args.ref, args.ref2, args.config)
    got = [answers(os.path.join(args.dir, f"{n}.jsonl")) for n in names]
    (ref, _), (ref2, _), (cfg, _) = got
    # A substituted pick is never sent to the judge as anyone's card (#1010).
    for n, (_, sub) in zip(names, got):
        if sub:
            print(f"judge {args.config}: {n}: {len(sub)} substituted {'pick' if len(sub) == 1 else 'picks'} left out "
                  f"({', '.join(f'{i:04d}' for i in sorted(sub))})", file=sys.stderr)
    todo = [i for i in sorted(cfg) if i in ref and ref.get(i) == ref2.get(i) and cfg[i] != ref[i]]
    path = os.path.join(args.dir, f"judge-{args.config}.jsonl")
    done = {}
    if os.path.exists(path):
        for line in open(path):
            r = json.loads(line)
            # A verdict is on two cards, and stands only while those are
            # still the answers: one judged before a substituted pick was
            # asked again (#1010's review) is on a card nobody chose.
            i = r["position"]
            if r.get("verdict") and cfg.get(i) == r.get("guide_card") and ref.get(i) == r.get("ref_card"):
                done[i] = r
    rest = [i for i in todo if i not in done]
    print(f"judge {args.config}: {len(todo)} changed settled picks, {len(rest)} to judge", file=sys.stderr)
    with cf.ThreadPoolExecutor(args.jobs) as pool, open(path, "a") as out:
        for r in pool.map(lambda i: judge(args, i, pos[i], cfg[i], ref[i]), rest):
            out.write(json.dumps(r) + "\n")
            out.flush()
            if r["verdict"]:
                done[r["position"]] = r
    rows = [done[i] for i in todo if i in done]
    g = sum(r["verdict"] == "guide" for r in rows)
    f = sum(r["verdict"] == "ref" for r in rows)
    e = sum(r["verdict"] == "even" for r in rows)
    n = g + f
    share = g / n if n else 0
    half = 1.96 * math.sqrt(share * (1 - share) / n) if n else 0
    a_said = sum(r["said"] == "A" for r in rows)
    print(f"{args.config}: of {len(rows)} judged ({len(todo) - len(rows)} unanswered), the guide's pick was better "
          f"{g}, worse {f}, even {e}; better in {share:.0%} ± {half:.0%} of decided picks; "
          f"the judge said A {a_said} times, B {sum(r['said'] == 'B' for r in rows)}")


if __name__ == "__main__":
    main()
