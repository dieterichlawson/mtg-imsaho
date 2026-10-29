#!/usr/bin/env python3
"""The nightly draft loop's notes across sessions: each seat's lessons from
tonight, and the guide rewritten from them and from every night's tables.

    scripts/draft-loop/reflect.py debrief --night DATE --out reports/draft-loop [--guide GUIDE]
    scripts/draft-loop/reflect.py curate  --night DATE --out reports/draft-loop --guide GUIDE

`debrief` asks every seat of `<out>/nights/<DATE>.json`, in one `claude -p`
call each, for up to three lessons grounded in what happened to it: its
picks, its deck, its matches, its own end-of-draft notes, and the guide it
drafted with (or that it drafted without one). They go to
`<out>/nights/<DATE>-lessons.json`.

`curate` hands the guide, tonight's lessons and `<out>/stats.md` (every
night's tables, `ledger.py stats`) to one call that rewrites the guide.
It is as long as it needs to be: no word count is set, and the curator is
asked to keep every line earning its place, since the guide is in every
guided seat's system prompt. The answer is taken when it has the six
headings once each (put in order if they came in another), ends with the
closing line or a closed fence, and fits in 64 KiB; otherwise the curator
is asked once more, told why, and then the old guide stays. The old guide
is kept in git.

The seats are `claude -p` on the CLI's own login — plan quota, never a
metered key: `DRAFT_LOOP_MODEL` (default `claude-haiku-5-5`, the model
that drafts) for the debriefs and `DRAFT_LOOP_CURATOR` (default
`claude-sonnet-5-5`: one call a night, and the guide is what every guided
seat reads) for the guide.
`CLAUDE_CODE_BIN` points at a stand-in CLI for a dry run.
"""
import argparse
import collections
import concurrent.futures as cf
import json
import os
import re
import subprocess
import sys
import uuid

SECTIONS = ["## Colours and archetypes", "## Commons that win", "## Uncommons and rares",
            "## Pick order", "## Building", "## Playing"]
# The line the curator ends the guide with, so a reply that stopped short is
# known for one (#967). It is not written to the guide.
END = "<!-- end of guide -->"
# The most the guide file may be. Every guided seat is sent it in its system
# prompt, which `claude -p` takes as one argument: Linux caps that at 128
# KiB, and the rules and card reference are about 41 KB of it (#965).
GUIDE_MAX_BYTES = 64 * 1024
HEADER = ("# Innistrad draft guide\n\n_Written by the nightly draft loop (`scripts/draft-loop/`) from "
          "{night}'s lessons and every night's statistics. Drafters read it at every pick._\n\n")


def claude(system, message, model, timeout=600):
    """One `claude -p` call: the answer's text and its usage."""
    env = {k: v for k, v in os.environ.items()
           if k not in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "CLAUDECODE") and not k.startswith("CLAUDE_CODE_SESSION")}
    env["FORCE_PROMPT_CACHING_5M"] = "1"
    env.setdefault("MAX_THINKING_TOKENS", "0")
    cmd = [os.environ.get("CLAUDE_CODE_BIN", "claude"), "-p", "--output-format", "json", "--tools", "",
           "--system-prompt", system, "--session-id", str(uuid.uuid4()), "--model", model, "--effort", "low"]
    for attempt in range(3):
        r = subprocess.run(cmd, input=message, capture_output=True, text=True, env=env, cwd="/tmp", timeout=timeout)
        try:
            out = json.loads(r.stdout)
        except json.JSONDecodeError:
            out = None
        if out and not out.get("is_error"):
            return out.get("result", ""), out.get("usage", {})
        print(f"claude -p failed (attempt {attempt + 1}): {(r.stderr or r.stdout)[-300:]}", file=sys.stderr)
    return None, {}


def json_in_text(text):
    """The whole text as JSON, or else the last object in it that parses:
    the first `{` to the last `}` was no object at all when a mana symbol
    like {G} sat in the prose around it (#896, as answer_text.rs reads)."""
    text = (text or "").strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass
    decoder, last, at = json.JSONDecoder(), None, 0
    while (i := text.find("{", at)) >= 0:
        try:
            value, end = decoder.raw_decode(text, i)
        except json.JSONDecodeError:
            at = i + 1
            continue
        if isinstance(value, dict):
            last = value
        at = end
    return last


def counted(cards):
    # "; " between names: "Grimgrin, Corpse-Born" is one card (#925).
    return "; ".join(f"{n}x {c}" if n > 1 else c for c, n in collections.Counter(cards).items())


def sideboarding(m, s):
    """What seat `s` changed from its registered deck for each game of match
    `m`, as "before game 2: in A; B, out C" (#901: the debrief told every
    seat its opponent's registered deck, whatever it played)."""
    # Against the deck of the game before, so a sideboard kept for games 2
    # and 3 is said once.
    before = collections.Counter(s["maindeck"])
    out = []
    for i, g in enumerate(m["games"], 1):
        deck = g.get("decks", {}).get(str(s["seat"]))
        if deck is None:
            continue
        deck = collections.Counter(deck)
        sided_in, sided_out = list((deck - before).elements()), list((before - deck).elements())
        if sided_in or sided_out:
            out.append(f"before game {i}: in {counted(sided_in) or 'nothing'}, out {counted(sided_out) or 'nothing'}")
        before = deck
    return out


def seat_story(night, s):
    """What happened to one seat, in the words the debrief is asked about."""
    seats = night["seats"]
    picks = collections.defaultdict(list)
    for pack, _, card in s["picks"]:
        picks[pack].append(card)
    lines = [f"You were seat {s['seat']} of {night['players']}. Packs 1 and 3 passed left, pack 2 right.",
             "Your picks, in order:"]
    lines += [f"  Pack {p}: " + "; ".join(cards) for p, cards in sorted(picks.items())]
    lines.append(f"Your deck ({s['colors']}): {counted(s['maindeck'])}. Basic lands: "
                 + ", ".join(f"{n} {l}" for l, n in s["lands"].items()) + ".")
    if s.get("notes"):
        lines.append(f"Your notes at the end of the draft: {s['notes']}")
    lines.append(f"Your record: {s['match_wins']}-{s['match_losses']}-{s['match_draws']} in matches, "
                 f"{s['game_wins']}-{s['game_losses']} in games.")
    for m in night["matches"]:
        if s["seat"] not in (m["a"], m["b"]):
            continue
        me_a = m["a"] == s["seat"]
        opp = seats[m["b"] if me_a else m["a"]]
        mine, theirs = (m["wins_a"], m["wins_b"]) if me_a else (m["wins_b"], m["wins_a"])
        # The games with no winner as a third figure, as the runner's score
        # says them: "lost 0-2" over three games left one out (#919).
        level = sum(1 for g in m["games"] if g.get("winner") is None)
        score = f"{mine}-{theirs}" + (f"-{level}" if level else "")
        turns = ", ".join(str(g["turns"]) for g in m["games"])
        lines.append(f"  Round {m['round']} vs seat {opp['seat']} ({opp['colors']}): "
                     f"{'won' if mine > theirs else 'lost' if mine < theirs else 'drew'} {score} (games of {turns} turns); "
                     f"their registered deck: {counted(opp['maindeck'])}")
        for who, seat in (("You", s), ("They", opp)):
            lines += [f"    {who} sideboarded {change}." for change in sideboarding(m, seat)]
    lines.append("The table: " + "; ".join(f"seat {o['seat']} {o['colors']} {o['match_wins']}-{o['match_losses']}" for o in seats) + ".")
    return "\n".join(lines)


DEBRIEF_SYSTEM = """You played tonight's Innistrad (ISD) booster draft: eight AI drafters, three packs of 14, then a
Swiss tournament of best-of-three matches with the decks drafted. You are asked what the next drafter of
this set should learn from what happened to you. Ground every lesson in this draft: a card that won or lost
games, a colour that flowed or dried up, a pick you would change, a deck-building choice. Name cards by their
exact names. A lesson that only restates general Limited advice is worth nothing."""


def debrief(args):
    night = json.load(open(os.path.join(args.out, "nights", f"{args.night}.json")))
    guide = open(args.guide).read().strip() if args.guide and os.path.exists(args.guide) else ""
    model = os.environ.get("DRAFT_LOOP_MODEL", "claude-haiku-5-5")

    def one(s):
        told = (f"You drafted with this guide in your prompt:\n\n{guide}\n" if s["guided"] and guide
                else "You drafted without a guide.\n")
        message = (f"{seat_story(night, s)}\n\n{told}\nWrite up to three lessons for the next drafter, each one "
                   "sentence of at most 35 words. Where tonight contradicted the guide, say so in the lesson.\n\n"
                   'Answer with only this JSON object: {"lessons": [string]}')
        text, usage = claude(DEBRIEF_SYSTEM, message, model)
        answer = json_in_text(text) or {}
        lessons = [l.strip() for l in answer.get("lessons", []) if isinstance(l, str) and l.strip()][:3]
        return {"seat": s["seat"], "guided": s["guided"], "colors": s["colors"],
                "record": f"{s['match_wins']}-{s['match_losses']}-{s['match_draws']}", "lessons": lessons, "usage": usage}

    with cf.ThreadPoolExecutor(4) as ex:
        results = list(ex.map(one, night["seats"]))
    path = os.path.join(args.out, "nights", f"{args.night}-lessons.json")
    json.dump({"night": args.night, "model": model, "seats": results}, open(path, "w"), indent=1)
    print(f"wrote {path}: {sum(len(r['lessons']) for r in results)} lessons from {len(results)} seats")


CURATE_SYSTEM = f"""You keep the draft guide for Innistrad (ISD) booster drafts played by AI drafters, eight to a
table, three packs of 14, best-of-three Swiss. The guide is put in front of every guided drafter at every
pick, so it must be specific and right, and as long as it needs to be and no longer: markdown under exactly
these headings, in this order: {", ".join(h[3:] for h in SECTIONS)}. Each night you get the guide, the lessons
tonight's drafters wrote, and the statistics of every night so far, and you write the next guide.

- Keep what the statistics or more than one lesson support; correct or drop what they contradict.
- A claim that rests on numbers says how many games it rests on, e.g. "(31 games)"; numbers from fewer than
  ten games are anecdotes and are said to be.
- Name cards exactly as the statistics and lessons spell them, and no others.
- Write for a drafter at a pick: what to take, what to pass, when to move in, how to build and play the deck.
- Length is yours to judge: add what the data supports, and cut what no longer earns its place. The guide
  travels in one command-line argument with the rules, so it may not pass {GUIDE_MAX_BYTES // 1024} KB (about 9,000 words).
- End the guide with the line {END}: an answer without it is taken to have been cut off."""


def sections(text):
    """A guide from its first section on: the title and the line the loop
    heads the file with are the loop's, not the curator's to copy (it did,
    and the guide had two titles). The first `## ` heading of any name, so
    a reply that starts with another of the six keeps it (#966)."""
    m = re.search(r"^## ", text, re.M)
    return text[m.start():].strip() if m else text.strip()


def unfenced(text):
    """The inside of the code fence a reply put its guide in, if it did:
    prose around the fence, a sign-off after it included, is the reply's
    and not the guide's. Only the whole reply was stripped of a fence
    before, so a fence with text after it went into the guide, fence and
    all (#967). Also whether it found the fence closed: a reply that closed
    its fence reached its end."""
    m = re.search(r"^```[a-z]*[ \t]*\n(.*?)^```[ \t]*$", text, re.S | re.M)
    return (m.group(1), True) if m and re.search(r"^## ", m.group(1), re.M) else (text, False)


def read_guide(reply):
    """The guide in a curator's reply, its six sections in the guide's order,
    and every reason it cannot be taken (none when it can).

    Headings are whole `## ` lines, not substrings: `### Playing` is not
    `## Playing`, and a section written twice is one too many. Sections in
    another order are put in order, not cut: everything before the first
    `## Colours and archetypes` used to be dropped, and the check then
    reported the cut headings as never written (#966). A reply that neither
    ends with the closing line nor closes a fence stopped short of its end, the way one cut
    off at its output limit does (#967); anything after that line is the
    reply's. And a guide longer than one argument can carry with the rules
    makes every guided pick fail (#965)."""
    text, closed = unfenced((reply or "").replace("\r\n", "\n").lstrip("\ufeff").strip())
    complete = closed or END in text
    text = text.split(END)[0]
    parts = re.split(r"^(## .*?)[ \t]*$", text, flags=re.M)
    found = [(parts[i], parts[i + 1].strip()) for i in range(1, len(parts), 2)]
    names = [h for h, _ in found]
    if not text.strip():
        return "", ["was empty"]
    problems = []
    missing = [h for h in SECTIONS if h not in names]
    if missing:
        problems.append(f"lacked the heading{'' if len(missing) == 1 else 's'} {', '.join(missing)}")
    twice = [h for h in SECTIONS if names.count(h) > 1]
    if twice:
        problems.append(f"had {', '.join(twice)} more than once")
    other = [h for h in names if h not in SECTIONS]
    if other:
        problems.append(f"had heading{'' if len(other) == 1 else 's'} not among the six: {', '.join(other)}")
    if not complete:
        problems.append(f"did not end with the line {END}, so it reads as cut off")
    guide = "\n\n".join(f"{h}\n\n{body}".strip() for h in SECTIONS for n, body in found if n == h)
    size = len((HEADER.format(night="2026-01-01") + guide + "\n").encode())
    if size > GUIDE_MAX_BYTES:
        problems.append(f"was {size:,} bytes, over the {GUIDE_MAX_BYTES:,} a guide may be")
    return guide, problems


def curate(args):
    stats_path = os.path.join(args.out, "stats.md")
    lessons_path = os.path.join(args.out, "nights", f"{args.night}-lessons.json")
    guide = sections(open(args.guide).read()) if os.path.exists(args.guide) else ""
    stats = open(stats_path).read() if os.path.exists(stats_path) else "(no statistics yet)"
    lessons = json.load(open(lessons_path)) if os.path.exists(lessons_path) else {"seats": []}
    told = "\n".join(f"- seat {s['seat']} ({s['colors']}, {s['record']}, {'guided' if s['guided'] else 'unguided'}): {l}"
                     for s in lessons["seats"] for l in s["lessons"])
    message = (f"The guide now:\n\n{guide or '(none yet: write the first one)'}\n\n"
               f"Tonight's lessons ({args.night}):\n{told or '(none)'}\n\n"
               f"The statistics of every night so far:\n\n{stats}\n\n"
               f"Write the next guide: only the markdown, starting with the first heading.")
    model = os.environ.get("DRAFT_LOOP_CURATOR", "claude-sonnet-5-5")
    # One more try for an answer that is not a guide, told why: the night's
    # lessons are not spent on a format slip.
    refused = []
    for attempt in range(2):
        text, usage = claude(CURATE_SYSTEM, message, model)
        new, problems = read_guide(text)
        words = len(new.split())
        missing = [h for h in SECTIONS if not re.search(f"^{re.escape(h)}[ \t]*$", new, re.M)]
        if not problems:
            break
        refused.append({"words": words, "missing_sections": missing, "problems": problems, "usage": usage})
        message += (f"\n\nYour last answer was not taken: it {'; it '.join(problems)}. Use exactly the six "
                    f"`## ` headings, each once, in order, and end with the line {END}.")
    record = {"night": args.night, "model": model, "attempts": attempt + 1, "words": words,
              "missing_sections": missing, "problems": problems, "usage": usage, "refused": refused}
    if problems:
        record["kept_old_guide"] = True
        record["rejected"] = text
        print(f"the new guide was not taken (it {'; it '.join(problems)}); the guide stays as it was", file=sys.stderr)
    else:
        open(args.guide, "w").write(HEADER.format(night=args.night) + new + "\n")
        print(f"wrote {args.guide}: {words} words")
    json.dump(record, open(os.path.join(args.out, "nights", f"{args.night}-curate.json"), "w"), indent=1)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("debrief", "curate"):
        p = sub.add_parser(name)
        p.add_argument("--night", required=True)
        p.add_argument("--out", required=True)
        p.add_argument("--guide", required=name == "curate")
    args = ap.parse_args()
    debrief(args) if args.cmd == "debrief" else curate(args)


if __name__ == "__main__":
    main()
