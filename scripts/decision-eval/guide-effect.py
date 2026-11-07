#!/usr/bin/env python3
"""What a draft guide changes about one night's picks.

    scripts/decision-eval/guide-effect.py DIR

`DIR` holds `positions.jsonl` (a night's kept picks), `guide-current.md`,
and four `draft-replay.py` results asked over those picks: `none-a` and
`none-b` under `--guide none`, `first` and `current` under two guides.
Two runs with no guide measure how often the model disagrees with itself;
a guide's effect is how much more often it changes a pick than that,
counted on the *settled* picks (where both no-guide runs agreed).

It prints how many picks each guide changes, by stage of the pack; how
many cheap-removal and skip-list cards (the current guide's own lists)
each configuration takes; the mean deck game win rate of the cards taken
(`reports/draft-loop/stats.json`, cards with 30+ games: a weak and partly
circular yardstick, since the guide is written from those numbers); and
the cards each guide takes more and less often. The changed picks go to
`DIR/changed.json`.
"""
import collections
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
D = sys.argv[1]
CONFIGS = ["none-a", "none-b", "first", "current"]
pos = [json.loads(l) for l in open(f"{D}/positions.jsonl")]
stats = {c["card"]: c for c in json.load(open(os.path.join(ROOT, "reports/draft-loop/stats.json")))["cards"]}
guide = open(f"{D}/guide-current.md").read()


def listed(heading_re):
    m = re.search(heading_re, guide, re.S)
    return {n.strip() for n in re.split(r",|\band\b", m.group(1)) if n.strip()} if m else set()


removal = listed(r"1\. Cheap removal: (.*?)\. Take")
skip = listed(r"Skip: (.*?)\. Delver")
skip = {re.sub(r"^and ", "", s) for s in skip} - {"Cloistered Youth over a flier", "Makeshift Mauler as a second copy"}

ans = {}
for c in CONFIGS:
    ans[c] = {}
    for l in open(f"{D}/{c}.jsonl"):
        r = json.loads(l)
        if r.get("action"):
            ans[c][int(r["position"])] = r["action"]

ids = sorted(set.intersection(*(set(a) for a in ans.values())))
settled = [i for i in ids if ans["none-a"][i] == ans["none-b"][i]]
print(f"positions answered by all four: {len(ids)} of {len(pos)}; settled without a guide (a == b): {len(settled)}")
print(f"no guide vs no guide (noise): {sum(ans['none-a'][i] != ans['none-b'][i] for i in ids) / len(ids):.0%} of picks differ")
for c in ("first", "current"):
    d_all = sum(ans[c][i] != ans["none-a"][i] for i in ids) / len(ids)
    d_set = sum(ans[c][i] != ans["none-a"][i] for i in settled) / len(settled)
    print(f"{c} guide vs no guide: {d_all:.0%} of all picks differ, {d_set:.0%} of settled picks")


def stage(i):
    n = len(pos[i]["available"])
    return "picks 1-4" if n >= 11 else ("picks 5-9" if n >= 6 else "picks 10-14")


print("\nchanged picks (of settled) by stage:")
for st in ("picks 1-4", "picks 5-9", "picks 10-14"):
    s = [i for i in settled if stage(i) == st]
    row = ", ".join(f"{c} {sum(ans[c][i] != ans['none-a'][i] for i in s) / max(len(s), 1):.0%}" for c in ("first", "current"))
    print(f"  {st} (n={len(s)}): {row}")


def wr(card):
    c = stats.get(card)
    return c["deck_game_win"] if c and c["games"] >= 30 else None


print("\nwhat each configuration picks (all answered positions):")
print(f"  {'config':8} {'removal':>8} {'skip-list':>9} {'mean deck win% (cards with 30+ games)':>40}")
for c in CONFIGS:
    picks = [ans[c][i] for i in ids]
    rem = sum(p in removal for p in picks)
    sk = sum(p in skip for p in picks)
    w = [wr(p) for p in picks if wr(p) is not None]
    print(f"  {c:8} {rem:8} {sk:9} {sum(w) / len(w):40.1%}")

for c in ("current", "first"):
    gained, lost = collections.Counter(), collections.Counter()
    for i in settled:
        if ans[c][i] != ans["none-a"][i]:
            gained[ans[c][i]] += 1
            lost[ans["none-a"][i]] += 1
    net = collections.Counter(gained)
    net.subtract(lost)
    print(f"\n{c}: cards it takes more than no guide does (settled picks): "
          + ", ".join(f"{k} +{v}" for k, v in net.most_common(10) if v > 0))
    print(f"{c}: cards it takes less: "
          + ", ".join(f"{k} {v}" for k, v in sorted(net.items(), key=lambda kv: kv[1])[:10] if v < 0))

changed = [i for i in settled if ans["current"][i] != ans["none-a"][i]]
both = [(wr(ans["current"][i]), wr(ans["none-a"][i])) for i in changed]
both = [(a, b) for a, b in both if a is not None and b is not None]
if both:
    up = sum(a > b for a, b in both)
    print(f"\nwhere the current guide changed a settled pick and both cards have 30+ games ({len(both)}): "
          f"its card has the higher deck win rate {up} times, lower {len(both) - up - sum(a == b for a, b in both)}")

json.dump({"changed": [{"position": i, "pack": pos[i]["pack_number"], "pick": pos[i]["pick_index"],
                        "seat": pos[i]["seat"], "no_guide": ans["none-a"][i], "current": ans["current"][i],
                        "first": ans["first"][i]} for i in changed]},
          open(f"{D}/changed.json", "w"), indent=1)
