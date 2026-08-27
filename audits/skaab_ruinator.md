## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/77/skaab-ruinator?utm_source=api
**Type line**: `Creature — Zombie Horror` — {1}{U}{U}, 5/6
**Oracle text**:
```
As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
```
**Status**: ISSUE

### Code issues
See below.

Same dead `on_resolve`; removed. Two further clauses checked:
- "You may cast this card from your graveyard" is a *permission*, correctly
  expressed by the cast being offered from the graveyard rather than by a
  yes/no prompt.
- The additional cost exiles three creature cards, paid at cast time.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
