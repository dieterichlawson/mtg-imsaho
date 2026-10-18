#!/usr/bin/env python3
"""Score replayed decisions against a reference configuration.

    scripts/decision-eval/compare.py --out DIR --ref NAME [--positions DIR] NAME...

Reads `<out>/<name>.jsonl` written by `replay.py`. For each position the
reference's answer is its most common action over its samples; a position
is *settled* when every reference sample agreed, which is the subset worth
scoring a candidate on (where the reference itself is split, a candidate
that differs has not been shown to play worse). For each configuration it
prints how often its action matches the reference — on settled positions
and on all of them — and what a decision cost it: model calls, output and
thinking tokens, system prompt and first-prompt bytes. With
`--positions` the disagreements are listed with object ids named from
the saves, for reading by hand.

Actions are compared as the engine's own `Action`, so two prompt formats
that offer the same choice differently compare equal. Sets (attackers,
blocks, discards, chosen objects) are compared unordered; an ordering is
compared in order; the tap plan a cast was offered with is ignored —
the decision is what to cast, not which identical land pays for it.
"""
import argparse
import collections
import json
import os

SET_LISTS = {"DeclareAttackers": "attackers", "DeclareBlockers": "assignments",
             "DiscardCards": "cards", "BottomCards": "cards"}
SET_TUPLES = {"ChosenSubset", "ChosenExileSet", "ChosenTargetSet", "ChosenObjectSet"}


def norm(action):
    if action is None:
        return None
    if isinstance(action, str):
        return action
    (kind, body), = action.items()
    body = json.loads(json.dumps(body))
    if kind == "CastSpell" and isinstance(body, dict):
        body.pop("tap_plan", None)
    if kind == "ActivateAbility" and isinstance(body, dict):
        body.pop("tap_plan", None)
    if kind in SET_LISTS and isinstance(body, dict):
        body[SET_LISTS[kind]] = sorted(body[SET_LISTS[kind]], key=json.dumps)
    if kind == "ResolveChoice" and isinstance(body, dict):
        choice = body.get("choice")
        if isinstance(choice, dict) and len(choice) == 1:
            (ck, cv), = choice.items()
            if ck in SET_TUPLES and isinstance(cv, list):
                choice[ck] = sorted(cv, key=json.dumps)
    return json.dumps({kind: body}, sort_keys=True)


def load(out, name):
    rows = collections.defaultdict(list)
    path = os.path.join(out, f"{name}.jsonl")
    for line in open(path):
        r = json.loads(line)
        rows[r["position"]].append(r)
    return rows


def namer(positions, position):
    if not positions:
        return lambda s: s
    try:
        state = json.load(open(os.path.join(positions, position + ".save")))["state"]
    except OSError:
        return lambda s: s
    names = {int(k): v["name"] for k, v in state["objects"].items()}

    def name(s):
        import re
        return re.sub(r'"object_id": (\d+)|\[(\d+), ', lambda m: m.group(0) + f"/*{names.get(int(m.group(1) or m.group(2)), '?')}*/", s)
    return name


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", required=True)
    ap.add_argument("--ref", required=True)
    ap.add_argument("--positions")
    ap.add_argument("--show", type=int, default=0, help="list up to N disagreements per configuration")
    ap.add_argument("names", nargs="+")
    args = ap.parse_args()

    ref = load(args.out, args.ref)
    modal, settled = {}, set()
    for pos, rs in ref.items():
        acts = [norm(r["action"]) for r in rs if r["action"] is not None]
        if not acts:
            continue
        top, n = collections.Counter(acts).most_common(1)[0]
        modal[pos] = top
        if n == len(acts) and len(acts) >= 1:
            settled.add(pos)
    ref_samples = max((len(v) for v in ref.values()), default=0)
    print(f"reference {args.ref}: {len(modal)} positions, {ref_samples} sample(s) each, "
          f"{len(settled)} settled (every sample agreed)")
    print()
    hdr = f"{'config':<22} {'n':>4} {'agree settled':>14} {'agree all':>10} {'calls':>6} {'out/dec':>8} {'think/dec':>9} {'sys B':>7} {'prompt B':>9} {'fallback':>8}"
    print(hdr)
    for name in args.names:
        rows = load(args.out, name)
        n = agree_s = n_s = agree_a = n_a = 0
        calls = out = think = fb = 0
        sysb, pb = [], []
        disagreements = []
        for pos, rs in rows.items():
            for r in rs:
                if r["action"] is None:
                    continue
                n += 1
                u = r["usage"]
                calls += u["calls"]; out += u["out"]; think += u["thinking"]
                fb += r["fallback"]
                if r["system_bytes"]: sysb.append(r["system_bytes"])
                if r["prompt_bytes"]: pb.append(r["prompt_bytes"])
                if pos in modal:
                    same = norm(r["action"]) == modal[pos]
                    n_a += 1; agree_a += same
                    if pos in settled:
                        n_s += 1; agree_s += same
                    if not same:
                        disagreements.append((pos, norm(r["action"]), modal[pos]))
        def pct(a, b):
            return f"{100 * a / b:5.1f}% {a}/{b}" if b else "-"
        avg = lambda xs: sum(xs) / len(xs) if xs else 0
        print(f"{name:<22} {n:>4} {pct(agree_s, n_s):>14} {pct(agree_a, n_a):>10} "
              f"{calls / max(n, 1):>6.2f} {out / max(n, 1):>8.0f} {think / max(n, 1):>9.0f} "
              f"{avg(sysb):>7.0f} {avg(pb):>9.0f} {fb:>8}")
        for pos, got, want in disagreements[: args.show]:
            nm = namer(args.positions, pos)
            print(f"    {pos}{' (settled)' if pos in settled else ''}\n      got  {nm(got)}\n      ref  {nm(want)}")


if __name__ == "__main__":
    main()
