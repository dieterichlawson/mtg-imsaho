# Continuation Prompt: Fix Remaining Audit Issues

## Context

We ran a full audit of all 249 Innistrad (ISD) cards using Sonnet 4.6. The audit reports are in `audits/sonnet46_2026_04_04/`. Each card has a `.md` file with status PASS, ISSUE, or SKIPPED.

**All behavioral bugs have been fixed.** The 54 verified bugs (16 engine + 30 card-specific) identified in `audits/fix_progress.md` are all resolved. All 865 tests pass with 0 failures.

**What remains: cosmetic/oracle-text issues across ~138 cards.** These are mismatches between the card's `oracle_text` field in its Rust implementation and the actual Scryfall oracle text. The user explicitly said "cosmetic oracle text should be correct" and "they should 100% match scryfall."

## What to do

### 1. Identify all remaining ISSUE cards

```bash
grep -l 'ISSUE' audits/sonnet46_2026_04_04/*.md
```

There are ~138 cards with ISSUE status. Many of these were already fixed (the behavioral bugs). For each ISSUE card:
- Read the audit file
- Check if the "Code issues" section mentions oracle text mismatches, type line issues, or other cosmetic problems
- Skip any behavioral issues that are already listed as fixed in `audits/fix_progress.md`

### 2. Common cosmetic issue types

From the audit, the most common cosmetic issues are:

- **Oracle text doesn't match Scryfall** — the `oracle_text` field in `card_data()` doesn't exactly match Scryfall's text. Fix by looking up the correct text with `python3 scripts/oracle_lookup.py lookup "Card Name"` and updating the string.
- **Missing or wrong subtypes** — e.g., missing "Human" from a Human creature's subtypes
- **Missing or wrong card types** — e.g., missing supertypes like "Legendary"
- **Wrong mana cost** — rare but exists
- **Missing keywords in the keywords vec** — e.g., a creature with flying that doesn't list `Keyword::Flying`

### 3. How to fix

For each card with a cosmetic issue:

1. Look up the correct oracle text: `python3 scripts/oracle_lookup.py lookup "Card Name"`
2. Read the card implementation: `mtg-engine/src/cards/isd/{card_name}.rs`
3. Update the `card_data()` method to match Scryfall exactly
4. Run tests to make sure nothing breaks: `cargo test -p mtg-engine`

### 4. Batch approach

Since there are ~138 cards, work in batches:
- Process 10-20 cards at a time
- For each batch, read the audit file, identify cosmetic issues, fix the card code
- Run `cargo test -p mtg-engine` after each batch
- Commit after each batch with a message like "Fix cosmetic oracle text for N cards"

### 5. Important rules

- **Do NOT change test assertions** unless the pre-existing test was testing incorrect behavior (per oracle text/rulings). If a test fails after your change, the card fix is wrong.
- **Oracle text must match Scryfall exactly** — including punctuation, capitalization, newlines, and special characters like em dashes.
- **Do NOT use training data for oracle text** — always fetch from the oracle cache or Scryfall API.
- **Commit frequently** — small batches, not one giant commit.

### 6. Files and tools

- Audit reports: `audits/sonnet46_2026_04_04/{card_name}.md`
- Card implementations: `mtg-engine/src/cards/isd/{card_name}.rs`
- Oracle lookup: `python3 scripts/oracle_lookup.py lookup "Card Name"` (or `fetch` if not cached)
- Test: `cargo test -p mtg-engine`
- Fix progress: `audits/fix_progress.md`

### 7. After all cosmetic fixes

Run a new audit pass on all cards to verify everything is PASS. Use the `/check-card isd` skill or run the batch audit script.
