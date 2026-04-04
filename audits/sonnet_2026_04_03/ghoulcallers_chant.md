## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Modal choice between 1 creature vs 2 zombies**: PASS - Engine correctly generates separate actions for each mode. Mode 0 creates single-target actions for any graveyard creature. Mode 1 creates two-target actions for pairs of zombies where t1 ≠ t2.
- **"target creature card from your graveyard"**: PASS - `is_valid_target` correctly checks `o.owner == caster` to restrict to your graveyard only.
- **"two target Zombie cards"**: PASS - Uses `TwoTargets` with `GraveyardCreatureOfSubtype("Zombie")` and engine enforces `t1 != t2` to prevent targeting same card twice.
- **Subtype checking for tokens**: PASS - `GraveyardCreatureOfSubtype` correctly checks both `o.subtypes` (runtime) and `registry.card_data().subtypes` (registry), so zombie tokens work correctly.
- **Targets leaving graveyard before resolution**: PASS - `on_resolve` checks `obj.zone == Zone::Graveyard` before moving each target, correctly handling fizzled targets.
- **Spell cleanup**: PASS - Correctly calls `state.move_spell_after_resolve(object_id)`.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Mode 1 returns one creature: `ghoulcallers_chant.rs:22`
- Mode 2 returns two zombies: `ghoulcallers_chant.rs:37`
- Legal actions include mode 1: `ghoulcallers_chant.rs:61`
- Legal actions include mode 2: `ghoulcallers_chant.rs:89`
- Mode 2 not available for non-zombies: `ghoulcallers_chant.rs:119`
- Cannot target opponent's graveyard: `ghoulcallers_chant.rs:161`
- Mixed graveyard modes work correctly: `ghoulcallers_chant.rs:183`
- Modal choice detection edge cases: NOT TESTED
- Zombie tokens targeting: NOT TESTED
- Targets leaving graveyard before resolution: NOT TESTED