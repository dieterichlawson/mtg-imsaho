## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: This creature deals 2 damage to target creature with flying.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
--- Back Face (Nightfall Predator) ---
{R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Archer Ranger Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- Engine never increments `spells_cast_this_turn` or populates `spells_cast_last_turn`, breaking both transform conditions in actual gameplay.
  - Oracle text says (front face): `At the beginning of each upkeep, if no spells were cast last turn, transform this creature.`
  - Oracle text says (back face): `At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.`
  - Code does: `state.spells_cast_last_turn` is declared in `state.rs:131` and read by `daybreak_ranger.rs:15-19`, but no code anywhere in the engine ever increments `state.spells_cast_this_turn` when a spell is cast (the `GameEvent::SpellCast` handler in `triggers.rs:644-676` only fires `SpellCastWatch` triggers, it never updates spell counts), and no code ever copies `spells_cast_this_turn` to `spells_cast_last_turn` at turn boundaries (`engine.rs:2882-2894` handles turn advancement with no spell-count transfer). Both fields remain at their initialized empty-HashMap values throughout actual gameplay. Consequence: the front-face condition `total_spells_last_turn == 0 && !state.is_first_turn` (`daybreak_ranger.rs:17`) evaluates to `true` on every non-first-turn upkeep (Daybreak Ranger always transforms regardless of spell casting), and the back-face condition `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (`daybreak_ranger.rs:19`) always evaluates to `false` (Nightfall Predator can never transform back).

### Tricky interactions checked

- **Front face target restriction ("target creature with flying")**: The `activated_abilities` for the front face uses `target_requirement: Some(TargetRequirement::Creature)`, and `is_valid_target` (`daybreak_ranger.rs:131-133`) further restricts to `state.has_keyword(*id, Keyword::Flying, registry)`. The `generate_ability_targets` function in `engine.rs:1297-1303` applies `TargetRequirement::Creature` pre-filter then calls `behavior.is_valid_target(...)` — flying restriction correctly enforced. PASS
- **Upkeep trigger fires for both faces**: `trigger_description` in `triggers.rs:311-327` checks front-face triggers first; finds `TriggerKind::Upkeep` with description "transform" on the front face. This non-empty description causes an `UpkeepTrigger` to be created for Nightfall Predator as well (even though `back_face_data().triggered_abilities` is empty), and `on_upkeep` correctly dispatches to the right condition via the `is_transformed` flag. PASS (functionally correct, though the design relies on the front-face trigger description as a shared sentinel)
- **`should_transform` respects `is_first_turn`**: `werewolf_should_transform` at `daybreak_ranger.rs:17` guards `&& !state.is_first_turn` to prevent turn-1 transforms. PASS
- **Transform condition uses per-player counts for back face**: Back face checks `state.spells_cast_last_turn.values().any(|&count| count >= 2)` — this is "any single player cast 2+", which correctly matches oracle "if a player cast two or more spells." PASS (logic is correct; the data is never populated — see Issue 1)
- **Fight ability deals damage to both fighters**: `combat::fight` in `combat.rs:158-168` deals `effective_power(a)` damage to `b` and `effective_power(b)` damage to `a`. Oracle: "Each deals damage equal to its power to the other." PASS
- **Nightfall Predator may fight own creature (no "another" restriction)**: Oracle says "fights target creature" with no controller restriction. `is_valid_target` for the transformed state returns `true` for any creature on the battlefield. PASS
- **"May" vs mandatory**: Both transform triggers are written as mandatory ("transform this creature") when the condition holds, with no "you may" wording. `on_upkeep` unconditionally calls transform when `should_transform` returns true. PASS
- **Dynamic P/T for back face**: `dynamic_pt` returns `Some((4, 4))` when `is_transformed`, and `None` otherwise. `effective_power`/`effective_toughness` in `state.rs:868,912` use the `dynamic_pt` return value in place of `obj.power`/`obj.toughness`. Nightfall Predator correctly resolves to 4/4. PASS
- **`spells_cast_last_turn` tracking (gameplay path)**: FAIL — see Issue 1. The engine never increments or transfers these counts, so transform conditions cannot operate correctly during actual play.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Daybreak Ranger transforms when no spells cast: `werewolf_cards.rs:376` (`daybreak_ranger_transforms_to_nightfall_predator`)
- Daybreak Ranger does NOT transform when spells were cast last turn: NOT TESTED (no Daybreak Ranger-specific test; Reckless Waif equivalent at `werewolf_cards.rs:61` but requires working engine spell tracking)
- Nightfall Predator transforms back when 2+ spells cast last turn: NOT TESTED (no Daybreak Ranger-specific test; Reckless Waif equivalent at `werewolf_cards.rs:74`)
- Front face activated ability exists: `werewolf_cards.rs:392` (`daybreak_ranger_has_activated_ability_on_front_face`)
- Front face target restricted to creatures with flying: NOT TESTED (test only checks ability description contains "flying", not that non-flying creatures are ineligible)
- Back face fight ability exists: `werewolf_cards.rs:404` (`nightfall_predator_has_fight_ability`)
- Nightfall Predator can fight own creature (no "another" restriction): `werewolf_cards.rs:417` (`nightfall_predator_can_fight_own_creature`)
- Nightfall Predator cannot fight opponent's creature — NOT TESTED (no test for targeting opponent creatures)
- Transform does not fire on first turn: NOT TESTED for Daybreak Ranger specifically (Reckless Waif equivalent at `werewolf_cards.rs:47`)
- Dynamic P/T correct for both faces: `werewolf_cards.rs:381-388` (checks 2/2 before transform and 4/4 after)
- `spells_cast_this_turn` incremented in real gameplay: NOT TESTED (all werewolf tests manually inject `spells_cast_last_turn`)
- `spells_cast_last_turn` populated from `spells_cast_this_turn` at turn end: NOT TESTED
