## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
**Type line**: Sorcery
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:33

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Cost reduction floor at {R}: PASS — `modified_cost` uses `.min(8)` on creature count, so generic portion floors at 0, leaving minimum cost of `{R}`. Matches ruling: "Blasphemous Act's ability can't reduce the total cost to cast the spell below {R}."
- Non-targeting (bypasses hexproof/shroud): PASS — `on_resolve` iterates all battlefield creatures without any target selection; no `targets` parameter is used. Correctly does not target.
- Damage is non-combat: PASS — emits `GameEvent::NonCombatDamageDealt` for each creature, which is the correct event type for spell damage (as opposed to `CombatDamageDealt`).
- Cost locked at cast time: PASS — `modified_cost` is called during `effective_spell_cost` at cast time (engine.rs:65), not at resolution. Creatures dying between cast and resolution do not change the paid cost.
- Damage source tracking: PASS — `obj.damaged_by.push(object_id)` correctly records the spell as damage source for triggers like Falkenrath Noble.

### Test coverage
- 13 damage to all creatures: `tier12_cards.rs:409` (blasphemous_act_deals_13_damage_to_all_creatures)
- Cost reduction with 0, 5, 10 creatures: `tier12_cards.rs:434` (blasphemous_act_cost_reduction)
- Castable with reduced cost ({R} with 8 creatures): `tier12_cards.rs:460` (blasphemous_act_castable_with_cost_reduction)
- Protection from red prevents damage: NOT TESTED
- Indestructible creatures survive 13 damage: NOT TESTED
