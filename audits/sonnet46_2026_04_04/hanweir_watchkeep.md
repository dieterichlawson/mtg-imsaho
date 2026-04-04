## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Defender\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Back face oracle text**: This creature attacks each combat if able.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Warrior Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- Engine never updates `spells_cast_this_turn` or `spells_cast_last_turn`, causing both werewolf transform conditions to evaluate incorrectly in actual gameplay.
  - Oracle text says (front face): `At the beginning of each upkeep, if no spells were cast last turn, transform this creature.`
  - Oracle text says (back face): `At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.`
  - Code does: `state.spells_cast_this_turn` is declared in `state.rs:127` and initialized to an empty HashMap at `state.rs:230`, but the engine's `Action::CastSpell` handler in `engine.rs` (lines 1479–1666) never increments it when a spell is cast. The `advance_step` function in `engine.rs` (lines 2867–2903) never copies `spells_cast_this_turn` into `spells_cast_last_turn` at turn end. Both fields remain empty HashMaps throughout any actual game. As a result, `werewolf_should_transform` at `hanweir_watchkeep.rs:10–18` evaluates:
    - Front face: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` always equals 0; condition `total_spells_last_turn == 0 && !state.is_first_turn` is always `true` after turn 1 → Hanweir Watchkeep always transforms on every upkeep even when spells were cast last turn.
    - Back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` is always `false` (HashMap is empty) → Bane of Hanweir never transforms back, regardless of how many spells were cast.

### Tricky interactions checked

- **Front-face transform condition ("if no spells were cast last turn")**: FAIL — `spells_cast_last_turn` is always empty; the condition is permanently satisfied after turn 1, so the creature always transforms even when spells were cast.
- **Back-face transform condition ("if a player cast two or more spells last turn")**: FAIL — `spells_cast_last_turn` is always empty; the condition is never satisfied, so Bane of Hanweir can never transform back.
- **First-turn guard (`!state.is_first_turn`)**: PASS — the `is_first_turn` flag is correctly checked in `werewolf_should_transform` (line 14) and cleared in `advance_step` (line 2887), preventing transformation on the very first turn.
- **Upkeep trigger fires for both faces**: PASS — `trigger_description` in `triggers.rs:311–327` checks the front face triggers first, returning "transform" for both the untransformed and transformed states. The trigger is dispatched unconditionally (desc is non-empty) and `on_upkeep` determines direction via `is_transformed`. This design is non-obvious but correct for the purpose of triggering.
- **Defender keyword on front face / absent on back face**: PASS — `has_keyword` in `state.rs:987–1043` checks `back_face_data().keywords` when `is_transformed=true`. The back face (`back_face_data()`) has `keywords: vec![]`, so Bane of Hanweir correctly has no Defender.
- **ForceAttack on back face / absent on front face**: PASS — `has_continuous_effect` in `state.rs:772–808` uses `back_face_data().continuous_effects` when `is_transformed=true`. The back face declares `ContinuousEffect::ForceAttack { scope: EffectScope::OnSelf }`, so Bane of Hanweir is correctly forced to attack each combat.
- **Defender prevents ForceAttack (front face can't attack)**: PASS — `engine.rs:1833–1835` checks Defender before adding a creature to the forced attackers list, so Hanweir Watchkeep (Defender on front face) is never forced to attack.
- **P/T via dynamic_pt**: PASS — `dynamic_pt` at `hanweir_watchkeep.rs:72–78` returns `Some((5, 5))` when transformed (Bane of Hanweir) and `None` otherwise (falls back to `card_data().power/toughness` = 1/5 for Hanweir Watchkeep).
- **Zone check before transforming**: PASS — `on_upkeep` at line 81 returns early if the object is not on the battlefield.
- **Intervening-if timing (check at trigger event and at resolution)**: PASS for structure — the condition in `on_upkeep` is the resolution check; the trigger fires unconditionally at upkeep start. However, this point is moot given the engine bug that makes the condition always evaluate incorrectly.
- **Transform log message when going back (Bane of Hanweir → Hanweir Watchkeep)**: Minor inaccuracy — `on_upkeep:89` hardcodes the log as `"Hanweir Watchkeep transforms into {}"`. When transforming back, the log emits `"Hanweir Watchkeep transforms into Hanweir Watchkeep"` instead of `"Bane of Hanweir transforms into Hanweir Watchkeep"`. No behavioral impact (game state is set correctly), and the back-to-front direction never fires in practice due to the engine bug above.
- **"Transform" and "Defender" listed as Scryfall keywords**: Not an issue — "Transform" is an ability word / keyword action, not a keyword ability tracked in the engine's `Keyword` enum. `Keyword::Defender` is correctly present in `card_data().keywords`.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- **Front-face transforms when no spells cast**: `werewolf_cards.rs:174` (`hanweir_watchkeep_loses_defender_gains_force_attack`) — TESTED (but test manipulates state directly, not via actual gameplay spell casting)
- **Front-face does NOT transform when spells were cast**: NOT TESTED for Hanweir Watchkeep; tested for Reckless Waif at `werewolf_cards.rs:61` (sets `spells_cast_last_turn` manually)
- **Back-face transforms back when 2+ spells cast**: NOT TESTED for Hanweir Watchkeep/Bane of Hanweir
- **Back-face does NOT transform back with only 1 spell cast**: NOT TESTED for Hanweir Watchkeep; tested for Reckless Waif at `werewolf_cards.rs:663`
- **Engine actually increments `spells_cast_this_turn` on spell cast**: NOT TESTED (no integration test goes through a full game loop, casting spells, then checking werewolf transform behavior)
- **Engine copies `spells_cast_this_turn` to `spells_cast_last_turn` at turn end**: NOT TESTED
- **Defender keyword present on front face**: `werewolf_cards.rs:180` — TESTED
- **Defender keyword absent on back face**: `werewolf_cards.rs:188` — TESTED
- **ForceAttack continuous effect on back face**: `werewolf_cards.rs:191–197` — TESTED
- **First-turn guard (no transform on turn 1)**: `werewolf_cards.rs:48` (Reckless Waif) — TESTED for another werewolf; not explicitly for Hanweir Watchkeep
- **P/T as 1/5 on front and 5/5 on back**: `werewolf_cards.rs:181,187` — TESTED
