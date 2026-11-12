#!/usr/bin/env python3
"""One night of the draft loop, for reading: `<out>/nights/<DATE>.md`.

    scripts/draft-loop/report.py --night DATE --out reports/draft-loop

From the files the night wrote (`ledger.py night`, `reflect.py debrief` and
`curate`): the standings with each seat's colours and whether it had the
guide (the loop's, the conventional one, or none), tonight's record for each, every seat's lessons, what
became of the guide, and what the night spent.
"""
import argparse
import json
import os


def count(n, one, many=None):
    """`n` and the noun for it: "1 call", "8 calls" (the runner's
    `mtg_draft_runner::count`, #914; this report said "curator: 1 calls"
    every night, #947)."""
    return f"{n} {one if n == 1 else (many or one + 's')}"


# Which guide a seat drafted with; a night from before seats recorded it
# had the loop's guide on every guided seat.
def guide_of(s):
    return s.get("guide", "loop" if s["guided"] else None)


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
    # The runner's own order where the ledger has it (points, then OMW, GW,
    # OGW); a night recorded before it is ranked as it was.
    if all("rank" in s for s in seats):
        order = sorted(seats, key=lambda s: s["rank"])
    else:
        order = sorted(seats, key=lambda s: (-(3 * s["match_wins"] + s["match_draws"]), -s["game_wins"], s["seat"]))
    def with_(g):
        n = [str(s["seat"]) for s in seats if guide_of(s) == g]
        return f"{'seat' if len(n) == 1 else 'seats'} {', '.join(n)}" if n else ""
    said = ([f"{with_('loop')} drafted with the guide (hash `{night['guide_hash']}`)"] if with_("loop") else []) \
        + ([f"{with_('conventional')} with the conventional guide (hash `{night.get('conventional_hash')}`)"]
           if with_("conventional") else []) \
        + ([f"{with_(None)} without one"] if with_(None) else [])
    out = [f"# Draft loop, {args.night}", "",
           f"Seed {night['seed']}, {night['players']} seats on `{seats[0]['model']}`, best-of-{night['best_of']}; "
           + ("; ".join(said) if with_("loop") or with_("conventional") else "no seat drafted with a guide") + ".", "",
           "| # | seat | guide | colours | matches | games | deck notes |", "| ---: | ---: | --- | --- | ---: | ---: | --- |"]
    for i, s in enumerate(order, 1):
        flags = []
        if s.get("fallback_deck"):
            flags.append("substitute deck")
        if s.get("substituted_picks"):
            flags.append(count(s['substituted_picks'], 'pick') + " substituted")
        out.append(f"| {i} | {s['seat']} | {({'loop': 'yes', 'conventional': 'conventional'}).get(guide_of(s), 'no')} | {s['colors']} | "
                   f"{s['match_wins']}-{s['match_losses']}-{s['match_draws']} | {s['game_wins']}-{s['game_losses']} | {', '.join(flags)} |")
    for label, want in (("guided", "loop"), ("conventional", "conventional"), ("unguided", None)):
        g = [s for s in seats if guide_of(s) == want]
        if g:
            out.append(f"\nTonight the {label} seats went {sum(s['match_wins'] for s in g)}-{sum(s['match_losses'] for s in g)} "
                       f"in matches and {sum(s['game_wins'] for s in g)}-{sum(s['game_losses'] for s in g)} in games.")
    h = night.get("health")
    subs = sum(s.get("substituted_picks", 0) for s in seats)
    decks = sum(1 for s in seats if s.get("fallback_deck"))
    if h:
        out.append(f"\nWhat went wrong: {count(h['reasks'], 're-ask')}, {count(h['unreadable'], 'reply', 'replies')} no answer could be read from, "
                   f"{count(h['panics'], 'panic, fatal stop or invariant break', 'panics, fatal stops or invariant breaks')}, "
                   f"{count(h.get('harness_chose', 0), 'game answer')} the harness chose for a seat, "
                   f"{count(subs, 'pick')} and {count(decks, 'deck')} substituted for a seat"
                   # A resume under another guide, setting or set (#948, #949).
                   + (f", {count(h['prompt_unlike'], 'resume line')} saying the night was asked under two system prompts"
                      if h.get("prompt_unlike") else "") + ". "
                   f"The games made {count(h['game_calls'], 'call')}: {count(h['out'], 'output token')} ({h['thinking']} thinking), "
                   f"{h['cache_read']} read from the cache, {h['cache_write']} written to it.")
        # What each re-ask was for, so a question that invited the wrong
        # answer can be told from a model's slip (#992).
        reasons = h.get("reask_reasons") or []
        if reasons:
            out.append("\nThe re-asks, by reason:\n")
            for r in reasons:
                seen = f" First answer: `{r['answer']}`" if r.get("answer") else ""
                out.append(f"- {r['count']} \u00d7 {r['who']}: {r['reason']}.{seen}")
    out += ["", "## Lessons", ""]
    for s in lessons["seats"]:
        for l in s["lessons"]:
            out.append(f"- seat {s['seat']} ({s['colors']}, {s['record']}): {l}")
    out += ["", "## The guide", ""]
    def why(r):
        # What reflect.py found wrong; a record from before #966 has only
        # the headings it took to be missing.
        if r.get("problems"):
            return "it " + "; it ".join(r["problems"])
        return f"missing {', '.join(r['missing_sections'])}" if r.get("missing_sections") else ""
    if curate.get("kept_old_guide"):
        out.append(f"Not rewritten: the curator's answer was not a guide ({curate.get('words')} words; "
                   f"{why(curate)}). The guide stays as it was.")
    elif curate:
        out.append(f"Rewritten by `{curate['model']}` in {curate.get('attempts', 1)} attempt(s): "
                   f"{curate['words']} words. `git log -p reports/draft-loop/guide.md` shows what changed.")
    # Every answer not taken is in `refused`; when none was, the last of
    # them is the one the line above already said.
    refused = curate.get("refused", [])
    for r in refused[:-1] if curate.get("kept_old_guide") else refused:
        out.append(f"An answer was refused: {r['words']} words" + (f"; {why(r)}" if why(r) else "") + ".")
    if night.get("usage"):
        out += ["", "## What it spent", "", "```", *night["usage"], "```"]
    calls = [("debriefs", [s.get("usage") or {} for s in lessons["seats"]]),
             ("curator", [r.get("usage") or {} for r in refused]
              + ([] if curate.get("kept_old_guide") or not curate else [curate.get("usage") or {}]))]
    spent = [f"{label}: {count(len(us), 'call')}, " + " / ".join(
                f"{sum(u.get(k, 0) for u in us)} {short}" for k, short in
                (("output_tokens", "out"), ("cache_read_input_tokens", "cache_read"), ("cache_creation_input_tokens", "cache_write")))
             for label, us in calls if us]
    if spent:
        out += ["", "After the tournament, on the CLI's login: " + "; ".join(spent) + "."]
    open(os.path.join(d, f"{args.night}.md"), "w").write("\n".join(out) + "\n")
    print(f"wrote {os.path.join(d, args.night + '.md')}")


if __name__ == "__main__":
    main()
