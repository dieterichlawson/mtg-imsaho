#!/usr/bin/env python3
"""One night of the draft loop, for reading: `<out>/nights/<DATE>.md`.

    scripts/draft-loop/report.py --night DATE --out reports/draft-loop

From the files the night wrote (`ledger.py night`, `reflect.py debrief` and
`curate`): the standings with each seat's colours and whether it had the
guide, tonight's guided and unguided records, every seat's lessons, what
became of the guide, and what the night spent.
"""
import argparse
import json
import os


def load(path):
    return json.load(open(path)) if os.path.exists(path) else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--night", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    d = os.path.join(args.out, "nights")
    night = load(os.path.join(d, f"{args.night}.json"))
    lessons = load(os.path.join(d, f"{args.night}-lessons.json")) or {"seats": []}
    curate = load(os.path.join(d, f"{args.night}-curate.json")) or {}
    seats = night["seats"]
    order = sorted(seats, key=lambda s: (-(3 * s["match_wins"] + s["match_draws"]), -s["game_wins"], s["seat"]))
    out = [f"# Draft loop, {args.night}", "",
           f"Seed {night['seed']}, {night['players']} seats on `{seats[0]['model']}`, best-of-{night['best_of']}; "
           f"even seats drafted with the guide (hash `{night['guide_hash']}`), odd seats without it.", "",
           "| # | seat | guide | colours | matches | games | deck notes |", "| ---: | ---: | --- | --- | ---: | ---: | --- |"]
    for i, s in enumerate(order, 1):
        flags = []
        if s.get("fallback_deck"):
            flags.append("substitute deck")
        if s.get("substituted_picks"):
            flags.append(f"{s['substituted_picks']} picks substituted")
        out.append(f"| {i} | {s['seat']} | {'yes' if s['guided'] else 'no'} | {s['colors']} | "
                   f"{s['match_wins']}-{s['match_losses']}-{s['match_draws']} | {s['game_wins']}-{s['game_losses']} | {', '.join(flags)} |")
    for label, want in (("guided", True), ("unguided", False)):
        g = [s for s in seats if s["guided"] == want]
        if g:
            out.append(f"\nTonight the {label} seats went {sum(s['match_wins'] for s in g)}-{sum(s['match_losses'] for s in g)} "
                       f"in matches and {sum(s['game_wins'] for s in g)}-{sum(s['game_losses'] for s in g)} in games.")
    out += ["", "## Lessons", ""]
    for s in lessons["seats"]:
        for l in s["lessons"]:
            out.append(f"- seat {s['seat']} ({s['colors']}, {s['record']}): {l}")
    out += ["", "## The guide", ""]
    if curate.get("kept_old_guide"):
        out.append(f"Not rewritten: the curator's answer was not a guide ({curate.get('words')} words, "
                   f"missing {curate.get('missing_sections')}). The guide stays as it was.")
    elif curate:
        out.append(f"Rewritten by `{curate['model']}` in {curate.get('attempts', 1)} attempt(s): "
                   f"{curate['words']} words. `git log -p reports/draft-loop/guide.md` shows what changed.")
    if night.get("usage"):
        out += ["", "## What it spent", "", "```", *night["usage"], "```"]
    open(os.path.join(d, f"{args.night}.md"), "w").write("\n".join(out) + "\n")
    print(f"wrote {os.path.join(d, args.night + '.md')}")


if __name__ == "__main__":
    main()
