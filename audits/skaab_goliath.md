## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/76/skaab-goliath?utm_source=api
**Type line**: `Creature — Zombie Giant` — {5}{U}, 6/9
**Oracle text**:
```
As an additional cost to cast this spell, exile two creature cards from your graveyard.
Trample
```
**Status**: ISSUE

### Code issues
See below.

Same dead `on_resolve` as Makeshift Mauler; removed. Exiles two creature cards as an additional cost, paid at cast time.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
