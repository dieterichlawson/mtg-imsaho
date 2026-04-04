## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: 
Front face: {T}: This creature deals 2 damage to target creature with flying.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.

Back face: {R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Type line**: Creature — Human Archer Ranger Werewolf // Creature — Werewolf
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Transform trigger on both faces: pass - Front face declares `TriggerKind::Upkeep` trigger, and the `trigger_description` function always checks front face first, so both transform directions work correctly
- Targeting restriction for flying creatures: pass - Uses `state.has_keyword(*id, Keyword::Flying, registry)` which properly checks both static and temporary keyword grants
- Fight ability can target any creature: pass - When transformed, `is_valid_target` returns `true` without controller restrictions, correctly allowing targeting of any creature including own creatures
- Spell counting for transform conditions: pass - Uses `spells_cast_last_turn` tracking with correct thresholds (0 for front->back, 2+ for back->front)
- Transform timing "at beginning of each upkeep": pass - Triggers on any player's upkeep, not just controller's upkeep
- Damage from activated ability is non-combat: pass - Correctly uses `NonCombatDamageDealt` event and marks damage on creature
- Fight ability uses proper fight mechanics: pass - Calls `crate::combat::fight(state, object_id, *target_id, registry)` which handles mutual damage correctly
- Dynamic P/T handling for transform: pass - Front face 2/2 base, back face uses `dynamic_pt` to return 4/4 when transformed

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Front to back transform: `werewolf_cards.rs:376` (daybreak_ranger_transforms_to_nightfall_predator)
- Front face activated ability: `werewolf_cards.rs:392` (daybreak_ranger_has_activated_ability_on_front_face) 
- Back face fight ability: `werewolf_cards.rs:404` (nightfall_predator_has_fight_ability)
- Fight ability targeting own creatures: `werewolf_cards.rs:417` (nightfall_predator_can_fight_own_creature)
- Back to front transform (2+ spells): NOT TESTED (but similar logic tested for other werewolves)
- Flying creature targeting: NOT TESTED
- Transform timing on all players' upkeeps: NOT TESTED
- Damage marking and damage events: NOT TESTED