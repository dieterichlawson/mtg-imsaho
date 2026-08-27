## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/161/rolling-temblor?utm_source=api
**Type line**: `Sorcery` — {2}{R}
**Oracle text**:
```
Rolling Temblor deals 2 damage to each creature without flying.
Flashback {4}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "each creature without flying" — checks `state.has_keyword(Flying)`, so a
  creature *granted* flying is spared and one that lost it is hit.
- Damage goes through `damage::deal_damage` with `DamageKind::NonCombat`, so
  protection, prevention and deathtouch apply and no combat-damage event is
  emitted for a sorcery.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
