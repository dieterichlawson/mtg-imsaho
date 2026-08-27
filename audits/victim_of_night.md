## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/124/victim-of-night?utm_source=api
**Type line**: `Instant` — {B}{B}
**Oracle text**:
```
Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "non-Vampire, non-Werewolf, non-Zombie" — all three excluded, and a creature
  with any one of them is not a legal target: PASS
- A Human Werewolf's *front* face is a Werewolf, so it is excluded on both
  faces: PASS
- `has_subtype` covers granted subtypes, so a creature Olivia Voldaren turned
  into a Vampire becomes an illegal target (CR 608.2b): PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The three exclusions: `cards_removal.rs`, `subtype.rs`
