## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/196/mulch?utm_source=api
**Type line**: `Sorcery` — {1}{G}
**Oracle text**:
```
Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
```
**Status**: PASS

### Code issues
No issues found.

Reveals the top four, lands to hand and the rest to the graveyard, using `has_card_type(Land)`. Handles a library of fewer than four.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
