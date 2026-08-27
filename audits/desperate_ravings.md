## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/139/desperate-ravings?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
Draw two cards, then discard a card at random.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "Draw two cards, **then** discard a card at random" — draws first, then picks
  from the whole hand, so a just-drawn card is eligible. Picking from only the
  drawn pair would be the tempting mistake.
- The discard is random rather than chosen, matching the wording.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
