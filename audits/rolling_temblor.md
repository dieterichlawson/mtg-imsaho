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

"deals 2 damage to each creature without flying" — no targeting, so the
battlefield is enumerated at resolution and each non-flier is dealt damage
through `damage::deal_damage` with `DamageKind::NonCombat` (not
`CombatDamageDealt`). No creature can die mid-loop: state-based actions only
run when a player would receive priority, which is after resolution.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_removal_and_bounce.rs` — fliers survive, non-fliers take 2; `damage_pipeline.rs` covers the non-combat event kind.

## Audit — 2026-08-28 19:21

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Rolling Temblor"`, https://scryfall.com/card/isd/161/rolling-temblor
**Oracle text**:
```
Rolling Temblor deals 2 damage to each creature without flying.
Flashback {4}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Sorcery
**Mana cost**: {2}{R}   **Keywords**: Flashback
**Rulings**: 6, all the generic flashback ones.
**Status**: PASS (the word "each" gained its test)

### Code issues
No issues found in `mtg-engine/src/cards/isd/rolling_temblor.rs`.

`{2}{R}`, `CardType::Sorcery`, `flashback_cost: Some({4}{R}{R})`, oracle text verbatim, no
target requirement ("each" does not target).

The sweep is every battlefield creature, all controllers, filtered by
`!has_keyword(Flying)` through the characteristics layer (so a granted or Aura-given flying
exempts, and a transformed face's flying counts). Damage goes through
`damage::deal_damage(.., DamageKind::NonCombat, ..)` with the spell as source, so it is a
noncombat damage event, `damaged_by` records the spell, and protection/prevention apply.

Sequential per-creature dealing is unobservable as non-simultaneity here: events are queued and
SBAs do not run mid-resolution, and nothing in this pool changes flying in response to damage.

### Tricky interactions checked
- **Non-flyer takes 2, flyer takes 0**: PASS.
- **"Each" includes the caster's own creatures**: PASS, newly pinned — both damaged creatures
  used to be the opponent's, so an opponents-only version passed.
- **`damaged_by` records the spell**: PASS — Abattoir Ghoul-style death triggers see it.
- **A 2-toughness creature dies to it, after the sweep, at SBA**: pipeline behaviour.
- **Flying read through the layer**: a Spectral Flight-enchanted ground creature is exempt;
  the object-keyword prop in the test exercises the union.
- **Sorcery timing and flashback exile**: engine-side, pinned generically.

### Test coverage
- 2 to each non-flyer including your own; flyers exempt:
  `flashback.rs:341 rolling_temblor_damages_non_flyers` (extended)
- the spell records itself in `damaged_by`:
  `cards_shortcuts_taken.rs:292 rolling_temblor_records_damage_source`
- flashback cost matches print: `card_data_invariants.rs:1907` (sweep)

Mutation-checked: hitting opponents only fails the extended test; hitting fliers too fails it.

### Changes made
- `flashback.rs`: the own-creature assertion. No code change.
