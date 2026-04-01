#!/usr/bin/env python3
"""Oracle text lookup and cache management for MTG card auditing.

Usage:
  python3 scripts/oracle_lookup.py lookup "Card Name"
  python3 scripts/oracle_lookup.py add-card "Card Name" --mana-cost "{1}{R}" --type-line "Enchantment — Aura" --oracle-text "The oracle text..." --source "Scryfall via WebSearch" --source-url "https://..."
  python3 scripts/oracle_lookup.py add-card-json '{"name": "Card Name", "mana_cost": "{1}{R}", ...}'
  python3 scripts/oracle_lookup.py add-ruling "Card Name" --date "2011-09-22" --text "The ruling..." --source "Scryfall rulings via WebSearch" --source-url "https://..."
  python3 scripts/oracle_lookup.py list
  python3 scripts/oracle_lookup.py lookup-dfc "Card Name" (for double-faced cards, shows both faces)
"""

import argparse
import json
import os
import sys
from datetime import date

CACHE_PATH = os.path.join(os.path.dirname(__file__), "..", "data", "oracle_cache.json")


def load_cache():
    with open(CACHE_PATH, "r") as f:
        return json.load(f)


def save_cache(cache):
    with open(CACHE_PATH, "w") as f:
        json.dump(cache, f, indent=2, ensure_ascii=False)
        f.write("\n")


def normalize_name(name):
    """Normalize card name for case-insensitive lookup."""
    return name.strip().lower()


def find_card(cache, name):
    """Find a card by name (case-insensitive)."""
    norm = normalize_name(name)
    for card_name, data in cache["cards"].items():
        if normalize_name(card_name) == norm:
            return card_name, data
    return None, None


def find_rulings(cache, name):
    """Find rulings by card name (case-insensitive)."""
    norm = normalize_name(name)
    for card_name, rulings in cache["rulings"].items():
        if normalize_name(card_name) == norm:
            return card_name, rulings
    return None, None


def cmd_lookup(args):
    cache = load_cache()
    canonical_name, card = find_card(cache, args.name)
    if not card:
        print(f"NOT FOUND: '{args.name}' is not in the oracle cache.")
        print("Use 'add-card' to cache it after fetching from an external source.")
        sys.exit(1)

    print(f"Name: {card['name']}")
    print(f"Mana Cost: {card.get('mana_cost', 'N/A')}")
    print(f"Type Line: {card.get('type_line', 'N/A')}")
    if card.get("power") or card.get("toughness"):
        print(f"P/T: {card.get('power', '?')}/{card.get('toughness', '?')}")
    print(f"Oracle Text: {card.get('oracle_text', 'N/A')}")
    if card.get("keywords"):
        print(f"Keywords: {', '.join(card['keywords'])}")
    print(f"Source: {card.get('source', 'unknown')}")
    print(f"Source URL: {card.get('source_url', 'N/A')}")
    print(f"Cached: {card.get('cached_at', 'unknown')}")

    # Show back face if present
    if card.get("back_face"):
        bf = card["back_face"]
        print(f"\n--- Back Face ---")
        print(f"Name: {bf['name']}")
        print(f"Type Line: {bf.get('type_line', 'N/A')}")
        if bf.get("power") or bf.get("toughness"):
            print(f"P/T: {bf.get('power', '?')}/{bf.get('toughness', '?')}")
        print(f"Oracle Text: {bf.get('oracle_text', 'N/A')}")

    # Show rulings if any
    _, rulings = find_rulings(cache, args.name)
    if rulings:
        print(f"\n--- Rulings ({len(rulings)}) ---")
        for r in rulings:
            print(f"[{r.get('date', '?')}] {r['text']}")
            print(f"  Source: {r.get('source', '?')} — {r.get('source_url', 'N/A')}")


def cmd_add_card(args):
    cache = load_cache()

    card_data = {
        "name": args.name,
        "mana_cost": args.mana_cost,
        "type_line": args.type_line,
        "oracle_text": args.oracle_text,
        "power": args.power,
        "toughness": args.toughness,
        "source": args.source,
        "source_url": args.source_url,
        "cached_at": str(date.today()),
    }
    if args.keywords:
        card_data["keywords"] = [k.strip() for k in args.keywords.split(",")]

    cache["cards"][args.name] = card_data
    save_cache(cache)
    print(f"Cached: {args.name}")


def cmd_add_card_json(args):
    cache = load_cache()
    card_data = json.loads(args.json)
    name = card_data["name"]
    card_data.setdefault("cached_at", str(date.today()))
    cache["cards"][name] = card_data
    save_cache(cache)
    print(f"Cached: {name}")


def cmd_add_back_face(args):
    cache = load_cache()
    canonical_name, card = find_card(cache, args.name)
    if not card:
        print(f"NOT FOUND: '{args.name}' — add the front face first.")
        sys.exit(1)

    card["back_face"] = {
        "name": args.back_name,
        "type_line": args.type_line,
        "oracle_text": args.oracle_text,
        "power": args.power,
        "toughness": args.toughness,
    }
    save_cache(cache)
    print(f"Added back face '{args.back_name}' to {canonical_name}")


def cmd_add_ruling(args):
    cache = load_cache()

    ruling = {
        "date": args.date,
        "text": args.text,
        "source": args.source,
        "source_url": args.source_url,
        "cached_at": str(date.today()),
    }

    if args.name not in cache["rulings"]:
        cache["rulings"][args.name] = []

    # Deduplicate by text
    existing_texts = {r["text"] for r in cache["rulings"][args.name]}
    if ruling["text"] not in existing_texts:
        cache["rulings"][args.name].append(ruling)
        save_cache(cache)
        print(f"Added ruling for {args.name}")
    else:
        print(f"Ruling already cached for {args.name} (duplicate)")


def cmd_list(args):
    cache = load_cache()
    cards = sorted(cache["cards"].keys())
    if not cards:
        print("Cache is empty.")
        return
    print(f"{len(cards)} cards cached:")
    for name in cards:
        card = cache["cards"][name]
        rulings_count = len(cache.get("rulings", {}).get(name, []))
        r_str = f" ({rulings_count} rulings)" if rulings_count else ""
        print(f"  {name} — {card.get('type_line', '?')}{r_str}")


def main():
    parser = argparse.ArgumentParser(description="Oracle text cache manager")
    sub = parser.add_subparsers(dest="command")

    # lookup
    p = sub.add_parser("lookup", help="Look up a card")
    p.add_argument("name", help="Card name")

    # add-card
    p = sub.add_parser("add-card", help="Add a card to the cache")
    p.add_argument("name", help="Card name")
    p.add_argument("--mana-cost", required=True)
    p.add_argument("--type-line", required=True)
    p.add_argument("--oracle-text", required=True)
    p.add_argument("--power", default=None)
    p.add_argument("--toughness", default=None)
    p.add_argument("--keywords", default=None, help="Comma-separated keywords")
    p.add_argument("--source", required=True)
    p.add_argument("--source-url", required=True)

    # add-card-json
    p = sub.add_parser("add-card-json", help="Add a card from JSON string")
    p.add_argument("json", help="JSON object with card fields")

    # add-back-face
    p = sub.add_parser("add-back-face", help="Add back face to a DFC")
    p.add_argument("name", help="Front face card name")
    p.add_argument("--back-name", required=True)
    p.add_argument("--type-line", required=True)
    p.add_argument("--oracle-text", required=True)
    p.add_argument("--power", default=None)
    p.add_argument("--toughness", default=None)

    # add-ruling
    p = sub.add_parser("add-ruling", help="Add a ruling")
    p.add_argument("name", help="Card name")
    p.add_argument("--date", required=True, help="Ruling date (YYYY-MM-DD)")
    p.add_argument("--text", required=True, help="Ruling text")
    p.add_argument("--source", required=True)
    p.add_argument("--source-url", required=True)

    # list
    sub.add_parser("list", help="List all cached cards")

    args = parser.parse_args()
    if not args.command:
        parser.print_help()
        sys.exit(1)

    cmds = {
        "lookup": cmd_lookup,
        "add-card": cmd_add_card,
        "add-card-json": cmd_add_card_json,
        "add-back-face": cmd_add_back_face,
        "add-ruling": cmd_add_ruling,
        "list": cmd_list,
    }
    cmds[args.command](args)


if __name__ == "__main__":
    main()
