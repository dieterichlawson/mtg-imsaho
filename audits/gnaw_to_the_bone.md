## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/183/gnaw-to-the-bone?utm_source=api
**Type line**: `Instant` — {2}{G}
**Oracle text**:
```
You gain 2 life for each creature card in your graveyard.
Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "for each creature **card** in your graveyard" — filters `!o.is_token`
  (CR 109.1) and excludes itself.
- Emits `LifeChanged`, and only when the gain is non-zero, so no event is
  reported for a life total that did not move.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
