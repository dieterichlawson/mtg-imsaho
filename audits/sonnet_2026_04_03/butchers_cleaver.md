## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature gets +3/+0. As long as equipped creature is a Human, it has lifelink. Equip {3}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues
- Subtype checking only reads registry data, missing tokens (`butchers_cleaver.rs:15-18`)
  - Oracle text says: `As long as equipped creature is a Human, it has lifelink`
  - Code does: Only checks `registry.card_data(o.card_id).subtypes` but not `o.subtypes`. Human tokens equipped with Butcher's Cleaver will not get lifelink because tokens store subtypes on the object, not in the registry.
  
- "As long as" condition not continuously evaluated (`butchers_cleaver.rs:14`)
  - Oracle text says: `As long as equipped creature is a Human, it has lifelink` 
  - Code does: Evaluates the Human condition only once when equipment is attached via `update_effects()` in `on_activate_ability()`. If the equipped creature later becomes Human or stops being Human, the lifelink effect will not update because the condition is never re-evaluated.

### Tricky interactions checked
- Human tokens with Butcher's Cleaver: fail (code only checks registry subtypes, not object subtypes)
- Creature becoming Human after equipment attachment: fail (condition not re-evaluated)
- Creature losing Human subtype after equipment attachment: fail (condition not re-evaluated)
- Normal Human creatures from registry: pass (covered by existing tests)
- +3/+0 power boost regardless of type: pass (always applied)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic functionality with Human creature: `tier9_equipment.rs:281-295`
- Basic functionality with non-Human creature: `tier9_equipment.rs:264-278` 
- Human tokens equipped with Butcher's Cleaver: NOT TESTED
- Dynamic type changes after equipment attachment: NOT TESTED
- Continuous "as long as" evaluation: NOT TESTED