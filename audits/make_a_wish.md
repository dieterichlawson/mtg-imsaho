## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/192/make-a-wish?utm_source=api
**Type line**: `Sorcery` — {3}{G}
**Oracle text**:
```
Return two cards at random from your graveyard to your hand.
```
**Status**: PASS

### Code issues
No issues found.

- "Return two **cards** at random" — filters `!o.is_token` (CR 109.1) and
  excludes the spell itself, which is on the stack rather than in the graveyard
  while it resolves.
- Genuinely random via `shuffle`, and `take(2)` handles a graveyard with fewer
  than two cards without panicking.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
