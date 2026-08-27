## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/87/army-of-the-damned?utm_source=api
**Type line**: `Sorcery` — {5}{B}{B}{B}
**Oracle text**:
```
Create thirteen tapped 2/2 black Zombie creature tokens.
Flashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

Thirteen 2/2 black Zombie tokens with their subtype, created tapped. The tap is applied after creation rather than as an entering replacement; nothing in this set watches an entering creature's tapped state, so it is not observable here — noted rather than changed.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
