# Oracle Text Lookup

Look up the current oracle text and rulings for a Magic: The Gathering card.

## Arguments
- `$ARGUMENTS` — One or more card names, comma-separated (e.g., "Lightning Bolt" or "Lightning Bolt, Doom Blade")

## How this works

We maintain a local oracle text cache at `data/oracle_cache.json` managed by `scripts/oracle_lookup.py`. This cache is the **single source of truth** for audits — it eliminates WebSearch unreliability and prevents oracle text hallucination.

## Procedure (repeat for each card)

### Step 1: Check the local cache

```bash
python3 scripts/oracle_lookup.py lookup "Card Name"
```

If the card is cached, output the cached data and you're done.

### Step 2: If not cached, fetch from Scryfall API

```bash
python3 scripts/oracle_lookup.py fetch "Card Name"
```

This single command will:
- Fetch the card from Scryfall API (fuzzy name match)
- Parse all fields (name, mana cost, type line, oracle text, P/T, keywords)
- Automatically handle DFCs (fetches both faces)
- Fetch all rulings for the card
- Cache everything to `data/oracle_cache.json`
- Print the result

To re-fetch a card that's already cached (e.g., after errata):
```bash
python3 scripts/oracle_lookup.py fetch "Card Name" --force
```

To bulk-fetch an entire set:
```bash
python3 scripts/oracle_lookup.py fetch-set isd
```
This fetches every card in the set with rulings, respecting Scryfall rate limits (~100ms between requests). Already-cached cards are skipped unless `--force` is used.

### Step 3: If Scryfall API is unavailable, fall back to WebSearch

If `fetch` fails (proxy block, network error, etc.), manually search and cache:

1. WebSearch: `{card name} scryfall oracle text` (restrict to scryfall.com)
2. If that doesn't work: `{card name} MTG oracle text gatherer`
3. Last resort: `{card name} MTG card text type line` (any source)

Then manually cache:
```bash
python3 scripts/oracle_lookup.py add-card "Card Name" \
  --mana-cost "{1}{R}" \
  --type-line "Enchantment — Aura Curse" \
  --oracle-text "Exact oracle text here..." \
  --source "Scryfall via WebSearch" \
  --source-url "https://scryfall.com/card/set/number/card-name"
```

For creatures add `--power` and `--toughness`. For keywords add `--keywords "Flying,Hexproof"`.

For DFCs, also add the back face:
```bash
python3 scripts/oracle_lookup.py add-back-face "Front Face Name" \
  --back-name "Back Face Name" \
  --type-line "Creature — Werewolf" \
  --oracle-text "Back face oracle text..." \
  --power "4" --toughness "4"
```

For rulings found via WebSearch:
```bash
python3 scripts/oracle_lookup.py add-ruling "Card Name" \
  --date "2011-09-22" \
  --text "The exact ruling text..." \
  --source "Scryfall rulings via WebSearch" \
  --source-url "https://scryfall.com/card/set/number/card-name"
```

### Step 4: Output the result

Print the full card data and all rulings for the user.

## Other commands

```bash
python3 scripts/oracle_lookup.py list              # List all cached cards
python3 scripts/oracle_lookup.py fetch-set isd     # Bulk-fetch entire Innistrad set
```

## Important rules
- **NEVER use your training data as a source for oracle text.** Only cache text from external sources.
- **Every cache entry has a source URL** — this is mandatory, not optional.
- **Do not paraphrase** — oracle text must be verbatim from the source.
