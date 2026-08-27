## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/132/brimstone-volley?utm_source=api
**Type line**: `Instant` — {2}{R}
**Oracle text**:
```
Brimstone Volley deals 3 damage to any target.
Morbid — Brimstone Volley deals 5 damage instead if a creature died this turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Morbid — deals 5 damage **instead** if a creature died this turn" — one
  damage event of 5, not 3 plus 2, and the condition is read when the spell
  resolves rather than when it was cast: PASS
- "any target" — creature, player or planeswalker: PASS
- Damage goes through `deal_damage`, so protection, prevention and the
  planeswalker loyalty path (CR 120.3c) all apply: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both damage amounts and the morbid condition: `cards_burn_and_damage.rs`, `cards_morbid_and_ltb.rs`
- Planeswalker targeting: `damage_helper.rs:every_any_target_spell_can_point_at_a_planeswalker`
