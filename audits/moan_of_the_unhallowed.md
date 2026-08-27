## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/109/moan-of-the-unhallowed?utm_source=api
**Type line**: `Sorcery` — {2}{B}{B}
**Oracle text**:
```
Create two 2/2 black Zombie creature tokens.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

Creates its two Zombie tokens through `create_token_with_subtypes` with ['Zombie'], so they are Zombies for Unbreathing Horde and the rest of the tribal set rather than nameless 2/2s.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
