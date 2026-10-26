#!/usr/bin/env python3
"""The nightly draft loop's ledger: one night's results as data, and every
night's results as the tables the guide is curated from.

    scripts/draft-loop/ledger.py night --save SAVE --positions POS --night DATE \
        [--guided 0,2,4,6] [--guide GUIDE] [--usage OUT.TXT] [--log DRAFT.LOG] --out reports/draft-loop
    scripts/draft-loop/ledger.py stats --out reports/draft-loop

`night` reads the runner's `--save` snapshot (picks, decks, matches, each
seat's notes) and `MTG_DRAFT_POSITIONS` (every pack a seat was handed) and
writes `<out>/nights/<DATE>.json`: per seat, its colours, deck, record and
picks, with whether it drafted with the guide. `stats` reads every night
and writes `<out>/stats.md` and `<out>/stats.json`: guided against
unguided seats, the colour pairs, and per card how early it is taken, how
often it makes a deck and how its decks do with it.

Colours come from the mana costs in `data/oracle_cache.json` and rarity
from the set's print runs, the way `mtg-draft/src/set_data.rs` reads them.
Nothing here calls a model.
"""
import argparse
import collections
import glob
import hashlib
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
COLORS = "WUBRG"


def front(name):
    return name.split(" // ")[0]


def card_facts(set_code="isd"):
    """name -> {"colors": "WU", "rarity": "common", "cmc": 3, "type": "..."} for the set's cards."""
    oracle = json.load(open(os.path.join(ROOT, "data", "oracle_cache.json")))["cards"]
    runs = json.load(open(os.path.join(ROOT, "data", "sets", f"{set_code}.json")))["runs"]
    rarity = {}
    for run, cards in runs.items():
        for c in cards:
            if run.startswith("uncommon"):
                rarity[front(c)] = "uncommon"
            elif run.startswith("common"):
                rarity[front(c)] = "common"
    # A sheet that mixes rarities: fewer copies is rarer (set_data.rs, `rarities`).
    for prefix in ("rare", "dfc"):
        copies = collections.Counter(front(c) for run, cards in runs.items() if run.startswith(prefix) for c in cards)
        counts = sorted(set(copies.values()), reverse=True)
        ladder = ["common", "uncommon", "rare", "mythic"]
        ladder = ladder[len(ladder) - len(counts):] if prefix == "rare" else ladder[:len(counts)]
        for c, n in copies.items():
            rarity.setdefault(c, ladder[counts.index(n)])
    facts = {}
    for name, card in oracle.items():
        cost = card.get("mana_cost") or ""
        colors = "".join(c for c in COLORS if "{" + c + "}" in cost or re.search(r"\{[0-9]*/?" + c + r"\}", cost))
        cmc = sum(int(x) if x.isdigit() else 1 for x in re.findall(r"\{([^}]*)\}", cost) if x != "X")
        facts[front(name)] = {"colors": colors, "rarity": rarity.get(front(name), "?"), "cmc": cmc,
                              "type": card.get("type_line") or ""}
    return facts


BASIC = {"Plains": "W", "Island": "U", "Swamp": "B", "Mountain": "R", "Forest": "G"}


def deck_colors(maindeck, facts, lands=None):
    """The deck's colours, in WUBRG order: a colour with at least four of the
    deck's spells, or three on five of its basics; a lighter one is a
    splash and is left out."""
    n = collections.Counter()
    for card in maindeck:
        for c in facts.get(front(card), {}).get("colors", ""):
            n[c] += 1
    basics = collections.Counter({BASIC[k]: v for k, v in (lands or {}).items() if k in BASIC})
    main = [c for c, k in n.most_common() if k >= 4 or (k >= 3 and basics[c] >= 5)]
    return "".join(sorted(main[:2], key=COLORS.index)) or (n.most_common(1)[0][0] if n else "C")


def final_ranks(log):
    """Each seat's place in the runner's FINAL STANDINGS, as the log prints
    them: points, then the event's tiebreakers (OMW, GW, OGW). The report
    ranked by points and raw game wins, so seats tied on points stood in
    another order than the runner's (#883's review)."""
    ranks, reading = {}, False
    if not log or not os.path.exists(log):
        return ranks
    for line in open(log, errors="replace"):
        if line.rstrip().endswith("FINAL STANDINGS"):
            ranks, reading = {}, True
            continue
        m = re.match(r"(\d+)\. Seat (\d+) \u2014 ", line) if reading else None
        if m:
            ranks[int(m.group(2))] = int(m.group(1))
        elif reading and ranks:
            reading = False
    return ranks


def night(args):
    facts = card_facts()
    save = json.load(open(args.save))
    guided = {int(s) for s in args.guided.split(",") if s.strip()} if args.guided else set()
    positions = [json.loads(l) for l in open(args.positions)] if args.positions and os.path.exists(args.positions) else []
    seats = []
    for seat in range(save["players"]):
        deck = save["decks"][seat]["deck"] if save["decks"][seat] else {"maindeck": [], "lands": {}}
        picks = [p for p in save["picks"] if p["seat"] == seat]
        picks.sort(key=lambda p: (p["round"], p["pick"]))
        seats.append({
            "seat": seat,
            "model": save["seats"][seat].get("model"),
            "guided": seat in guided,
            "colors": deck_colors(deck["maindeck"], facts, deck.get("lands")),
            "maindeck": deck["maindeck"],
            "lands": deck.get("lands", {}),
            "fallback_deck": bool(save["decks"][seat].get("fallback")) if save["decks"][seat] else True,
            # [pack, pick within the pack, card], in the order taken.
            "picks": [[p["round"], p["pick"], p["card"]] for p in picks],
            "substituted_picks": sum(1 for p in picks if p.get("substituted")),
            "notes": save["notes"][seat] if save.get("notes") else None,
            "match_wins": 0, "match_losses": 0, "match_draws": 0, "game_wins": 0, "game_losses": 0, "game_draws": 0,
        })
    for seat, rank in final_ranks(args.log).items():
        if seat < len(seats):
            seats[seat]["rank"] = rank
    matches = []
    for m in save["matches"]:
        r = m["result"]
        a, b = r["player_a"], r["player_b"]
        games = [{"winner": g.get("winner"), "turns": g.get("turns")} for g in r["games"]]
        matches.append({"round": m["round"], "a": a, "b": b, "wins_a": r["wins_a"], "wins_b": r["wins_b"], "games": games})
        for g in games:
            w = g["winner"]
            for s in (a, b):
                key = "game_draws" if w is None else ("game_wins" if w == s else "game_losses")
                seats[s][key] += 1
        if r["wins_a"] == r["wins_b"]:
            seats[a]["match_draws"] += 1
            seats[b]["match_draws"] += 1
        else:
            win, lose = (a, b) if r["wins_a"] > r["wins_b"] else (b, a)
            seats[win]["match_wins"] += 1
            seats[lose]["match_losses"] += 1
    # What each seat was offered: for "taken over" and "seen" counts.
    offered = [{"seat": p["seat"], "pack": p["pack_number"], "pick": p["pick_index"],
                "available": p["available"], "chosen": p["chosen"]} for p in positions]
    guide_hash = None
    if args.guide and os.path.exists(args.guide):
        guide_hash = hashlib.sha256(open(args.guide, "rb").read()).hexdigest()[:16]
    record = {
        "night": args.night, "seed": save["seed"], "set": save["set"], "players": save["players"],
        "best_of": save["best_of"], "guide_hash": guide_hash, "seats": seats, "matches": matches,
        "offered": offered, "usage": usage_lines(args.usage) if args.usage else None,
        "health": log_health(args.log),
    }
    os.makedirs(os.path.join(args.out, "nights"), exist_ok=True)
    path = os.path.join(args.out, "nights", f"{args.night}.json")
    json.dump(record, open(path, "w"), indent=1)
    print(f"wrote {path}")


def log_health(path):
    """What the run's log says went wrong, and what its game calls spent:
    re-asks, replies no answer could be read from, panics and invariant
    breaks, and the `USAGE` lines of every game call (the runner's own
    summary misses the calls before a `--resume`)."""
    if not path or not os.path.exists(path):
        return None
    h = {"reasks": 0, "unreadable": 0, "panics": 0, "harness_chose": 0, "game_calls": 0,
         "out": 0, "thinking": 0, "cache_read": 0, "cache_write": 0}
    # A game seat's re-ask is `REASK [SeatN]`, a pick's a bare `REASK`; a
    # draft reply with no answer in it is the backend's `API_WARN ... no
    # structured object`, the game seat's `UNREADABLE`; and a call whose
    # result gave no thinking figure says `thinking n/a`. Each was missed,
    # so the night said "0 calls" beside the runner's 1274 (#900).
    usage = re.compile(r"\tUSAGE \[.*out (\d+) thinking (\d+|n/a) cache_read (\d+) cache_write (\d+)")
    for line in open(path, encoding="utf-8", errors="replace"):
        if re.search(r"\tREASK(\t| \[|$)", line):
            h["reasks"] += 1
        elif "UNREADABLE" in line or ("API_WARN" in line and "no structured object" in line):
            h["unreadable"] += 1
        # A worker's panic reaches the log as its `FATAL` line (the panic
        # text is stderr's), and so does a backend that gave up, after its
        # own `API_FATAL`, which is not counted again (#900's review).
        elif "panicked" in line or "INVARIANT" in line or re.search(r"\tFATAL\b", line):
            h["panics"] += 1
        # A game seat's answer the harness could not use and chose for.
        elif re.search(r"\t(MALFORMED|NO_ANSWER) \[", line):
            h["harness_chose"] += 1
        elif (m := usage.search(line)):
            h["game_calls"] += 1
            for k, v in zip(("out", "thinking", "cache_read", "cache_write"), m.groups()):
                h[k] += int(v) if v != "n/a" else 0
    return h


def usage_lines(path):
    """The runner's token summary, as the lines it printed."""
    if not os.path.exists(path):
        return None
    text = open(path, encoding="utf-8", errors="replace").read()
    i = text.rfind("=== Token Usage ===")
    if i < 0:
        return None
    lines = text[i:].strip().splitlines()[:12]
    return [l for l in lines if not l.startswith("Done. Log written")]


def pct(w, n):
    return f"{100 * w / n:.0f}%" if n else "-"


def stats(args):
    facts = card_facts()
    nights = [json.load(open(p)) for p in sorted(glob.glob(os.path.join(args.out, "nights", "*.json")))
              if re.fullmatch(r"\d{4}-\d\d-\d\d(_[a-z0-9]+)?\.json", os.path.basename(p))]
    if not nights:
        sys.exit("no nights recorded yet")
    group = collections.defaultdict(lambda: collections.Counter())
    pairs = collections.defaultdict(lambda: collections.Counter())
    cards = collections.defaultdict(lambda: collections.Counter())
    pick_sum = collections.Counter()
    for n in nights:
        for s in n["seats"]:
            g = group["guided" if s["guided"] else "unguided"]
            for k in ("match_wins", "match_losses", "match_draws", "game_wins", "game_losses", "game_draws"):
                g[k] += s[k]
                pairs[s["colors"]][k] += s[k]
            g["seats"] += 1
            pairs[s["colors"]]["decks"] += 1
            for _, pick, card in s["picks"]:
                c = cards[front(card)]
                c["picked"] += 1
                pick_sum[front(card)] += pick
            for card in set(s["maindeck"]):
                c = cards[front(card)]
                c["decks"] += 1
                c["game_wins"] += s["game_wins"]
                c["games"] += s["game_wins"] + s["game_losses"] + s["game_draws"]
        for o in n.get("offered", []):
            for card in set(o["available"]):
                cards[front(card)]["seen"] += 1
    out = ["# Draft loop: what the nights add up to", "",
           f"{len(nights)} night(s), {sum(len(n['seats']) for n in nights)} decks, "
           f"{sum(len(m['games']) for n in nights for m in n['matches'])} games. "
           "Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.", ""]
    out += ["## Guided against unguided seats", "",
            "Even seats draft with `guide.md`, odd seats without it, every night.", "",
            "| seats | n | matches W-L-D | match win | games W-L | game win |", "| --- | ---: | ---: | ---: | ---: | ---: |"]
    for k in ("guided", "unguided"):
        g = group[k]
        if not g["seats"]:
            continue
        mn = g["match_wins"] + g["match_losses"] + g["match_draws"]
        gn = g["game_wins"] + g["game_losses"] + g["game_draws"]
        out.append(f"| {k} | {g['seats']} | {g['match_wins']}-{g['match_losses']}-{g['match_draws']} | "
                   f"{pct(g['match_wins'] + 0.5 * g['match_draws'], mn)} | {g['game_wins']}-{g['game_losses']} | {pct(g['game_wins'], gn)} |")
    out += ["", "## Colour pairs", "", "A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.", "",
            "| colours | decks | matches W-L-D | game win |", "| --- | ---: | ---: | ---: |"]
    for k, p in sorted(pairs.items(), key=lambda kv: (-kv[1]["decks"], kv[0])):
        gn = p["game_wins"] + p["game_losses"] + p["game_draws"]
        out.append(f"| {k} | {p['decks']} | {p['match_wins']}-{p['match_losses']}-{p['match_draws']} | {pct(p['game_wins'], gn)} ({gn}) |")
    rows = []
    for name, c in cards.items():
        f = facts.get(name, {})
        rows.append({"card": name, "colors": f.get("colors", ""), "rarity": f.get("rarity", "?"),
                     "seen": c["seen"], "picked": c["picked"],
                     "avg_pick": round(pick_sum[name] / c["picked"], 1) if c["picked"] else None,
                     "decks": c["decks"], "games": c["games"], "game_wins": c["game_wins"],
                     "deck_game_win": round(c["game_wins"] / c["games"], 3) if c["games"] else None})
    json.dump({"nights": len(nights), "groups": group, "pairs": pairs, "cards": rows},
              open(os.path.join(args.out, "stats.json"), "w"), indent=1, default=dict)
    min_games = max(6, len(nights) * 2)
    out += ["", f"## Cards in winning decks (at least {min_games} games in a deck)", "",
            "`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.", "",
            "| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |", "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |"]
    for r in sorted((r for r in rows if r["games"] >= min_games), key=lambda r: -r["deck_game_win"])[:40]:
        out.append(f"| {r['card']} | {r['colors'] or 'C'} | {r['rarity']} | {r['decks']} | {r['games']} | {pct(r['game_wins'], r['games'])} | {r['avg_pick']} | {r['picked']}/{r['seen']} |")
    out += ["", "## Taken earliest (picked at least 3 times)", "",
            "| card | colours | rarity | avg pick | picked | decks |", "| --- | --- | --- | ---: | ---: | ---: |"]
    for r in sorted((r for r in rows if r["picked"] >= 3), key=lambda r: r["avg_pick"])[:30]:
        out.append(f"| {r['card']} | {r['colors'] or 'C'} | {r['rarity']} | {r['avg_pick']} | {r['picked']} | {r['decks']} |")
    open(os.path.join(args.out, "stats.md"), "w").write("\n".join(out) + "\n")
    print(f"wrote {os.path.join(args.out, 'stats.md')}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    n = sub.add_parser("night")
    n.add_argument("--save", required=True)
    n.add_argument("--positions")
    n.add_argument("--night", required=True)
    n.add_argument("--guided", default="")
    n.add_argument("--guide")
    n.add_argument("--usage")
    n.add_argument("--log")
    n.add_argument("--out", required=True)
    s = sub.add_parser("stats")
    s.add_argument("--out", required=True)
    args = ap.parse_args()
    night(args) if args.cmd == "night" else stats(args)


if __name__ == "__main__":
    main()
