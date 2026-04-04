## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
--- Back Face (Ironfang) ---
First strike
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- Engine never tracks spells cast per turn, so transform conditions are always wrong in a real game (`mtg-engine/src/engine.rs` — turn transition in `advance_step`, and `CastSpell` handler in `submit_action`)
  - Oracle text says (front face): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`
  - Oracle text says (back face): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`
  - Code does: The card reads `state.spells_cast_last_turn` (front face, `village_ironsmith.rs:12`: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum();`; back face, `village_ironsmith.rs:16`: `state.spells_cast_last_turn.values().any(|&count| count >= 2)`), but `spells_cast_last_turn` is initialized as an empty `HashMap` in `GameState::new()` (`state.rs:231`) and **is never populated** by the engine. No code in `engine.rs` increments `spells_cast_this_turn` when a spell is cast (the `CastSpell` branch at `engine.rs:1479` emits `GameEvent::SpellCast` but never touches these fields), and the turn-transition code in `advance_step` (`engine.rs:2882–2895`) never rotates `spells_cast_this_turn` into `spells_cast_last_turn`. In any real game run through the engine, both maps are always empty.
  - Consequence: `total_spells_last_turn` is always 0, so Village Ironsmith (front face) always transforms on upkeep after turn 1, even when spells were cast. `spells_cast_last_turn.values().any(|&count| count >= 2)` is always false, so Ironfang (back face) never transforms back, even when 2+ spells were cast.

- Incorrect log message when Ironfang transforms back to Village Ironsmith (`mtg-engine/src/cards/isd/village_ironsmith.rs:87–88`)
  - Oracle text: Ironfang transforms into Village Ironsmith (the source of the transform is Ironfang)
  - Code does: `format!("Village Ironsmith transforms into {}", name)` — the prefix is hardcoded as "Village Ironsmith" regardless of which face is currently showing. When Ironfang transforms back, the log prints "Village Ironsmith transforms into Village Ironsmith" instead of "Ironfang transforms into Village Ironsmith".

### Tricky interactions checked

- **Front face upkeep trigger fires correctly**: PASS — `trigger_description()` in `triggers.rs:314` checks front face `triggered_abilities` first and finds `TriggerKind::Upkeep` → description "transform" is returned → trigger is collected every upkeep for both transformed and non-transformed states.
- **Back face trigger fires despite empty `triggered_abilities`**: PASS — The back face `triggered_abilities: vec![]` is not needed because `trigger_description()` returns the front face's description first (before checking the back face). Both faces fire via the same front-face trigger entry, and `on_upkeep` dispatches to the correct transform direction based on `is_transformed`.
- **`on_upkeep` correctly guards against non-battlefield**: PASS — `village_ironsmith.rs:79` checks `zone != Zone::Battlefield` and returns early if the creature is not on the battlefield. Additionally, `triggers.rs:955` also checks `zone == Battlefield` before calling `on_upkeep`.
- **"each upkeep" fires for both players' upkeeps**: PASS — `collect_triggers` in `triggers.rs:605` iterates ALL battlefield permanents for `StepStarted { step: Upkeep }`, not filtered by active player. So Village Ironsmith triggers on every player's upkeep, as the oracle text requires.
- **"if no spells were cast last turn" — front face**: FAIL — `spells_cast_last_turn` is never populated in a real game (see main issue above). The condition is always met, causing always-transform.
- **"if a player cast two or more spells last turn" — per-player check**: The card's per-player check logic is structurally correct (`any(|&count| count >= 2)` requires one player to have ≥2 spells, matching "a player"), but the data is never populated, so it always returns false.
- **First strike present on both faces**: PASS — front face `card_data()` has `keywords: vec![Keyword::FirstStrike]`; back face `back_face_data()` also has `keywords: vec![Keyword::FirstStrike]`. `has_keyword()` in `state.rs:1006` checks the appropriate face based on `is_transformed`.
- **Dynamic P/T for Ironfang (3/1)**: PASS — `dynamic_pt()` returns `Some((3, 1))` when `is_transformed`, and `None` when not. `effective_power` and `effective_toughness` use this correctly as the base P/T.
- **Transform does not double-count P/T**: PASS — `dynamic_pt()` is only called in `continuous_pt_mods()` for sources `attached_to == Some(creature_id)`. Village Ironsmith is not an aura, so this path is not triggered for it. `effective_power` uses `dynamic_pt()` for the base P/T, not as an additive modifier.
- **First-turn prevention**: The code checks `!state.is_first_turn` (line 14) to prevent transformation on the first turn. This is consistent with all other werewolf implementations and tested behavior.
- **Transform toggling is correct**: PASS — `obj.is_transformed = !obj.is_transformed` correctly toggles, and the name is updated accordingly (`village_ironsmith.rs:85–86`).
- **Spell type restriction in `SpellCastWatch` does not affect werewolf tracking**: Confirmed independent — `collect_triggers` only dispatches `SpellCastWatch` for instants/sorceries (`triggers.rs:650`), but werewolf transform tracking is a separate mechanism using `spells_cast_last_turn`, not `SpellCastWatch`.

### Test coverage

- "if no spells were cast last turn, transform this creature" (front face) — tested by `werewolf_cards.rs:135` (`village_ironsmith_keeps_first_strike_on_both_faces`) but only the happy path (no spells cast, transforms). The case where Village Ironsmith should NOT transform when spells were cast is NOT TESTED for Village Ironsmith directly (only for Reckless Waif at `werewolf_cards.rs:61`).
- "if a player cast two or more spells last turn, transform this creature" (back face, Ironfang) — NOT TESTED for Village Ironsmith. No test verifies Ironfang transforms back when 2+ spells were cast.
- First strike on front face — `werewolf_cards.rs:141`
- First strike on back face (Ironfang) — `werewolf_cards.rs:149`
- 3/1 P/T on Ironfang — `werewolf_cards.rs:147–148`
- "each upkeep" fires for both players — NOT TESTED (no test exercises upkeep trigger from opponent's turn for Village Ironsmith specifically)
- Engine spell count tracking (core engine bug) — NOT TESTED in integration. Tests bypass this by manually setting `state.spells_cast_last_turn` (e.g., `werewolf_cards.rs:64`, `werewolf_cards.rs:84`). No end-to-end test verifies that casting spells through `submit_action` updates `spells_cast_this_turn` and that it rolls over to `spells_cast_last_turn` at turn transition.
- First-turn prevention — `werewolf_cards.rs:48` (for Reckless Waif); not explicitly tested for Village Ironsmith but the same `werewolf_should_transform` logic applies.
