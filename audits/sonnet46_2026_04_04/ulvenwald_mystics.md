## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of each upkeep, if no spells were cast last turn, transform this creature. // {G}: Regenerate this creature. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Shaman Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- Engine never increments `spells_cast_this_turn` and never transfers it to `spells_cast_last_turn`; both transform conditions are permanently wrong in real gameplay
  - File: `mtg-engine/src/engine.rs` CastSpell handler (lines 1479–1666): no increment of `spells_cast_this_turn` when a spell is cast; `advance_step` turn-end transition (lines 2867–2895): no rollover of `spells_cast_this_turn` into `spells_cast_last_turn`.
  - Oracle text says (front face): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`
  - Oracle text says (back face): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`
  - Code does (`mtg-engine/src/cards/isd/ulvenwald_mystics.rs` lines 15–19): `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum();` / `total_spells_last_turn == 0 && !state.is_first_turn` (front face) and `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (back face). Both read from `spells_cast_last_turn` which is initialized to `HashMap::new()` (`state.rs` line 231) and is never written anywhere in the engine. Consequence: `spells_cast_last_turn` is always empty in a real game; front face always sees "0 spells cast last turn" and transforms on every non-first-turn upkeep regardless of actual spell casts; back face always sees "no player cast 2+" and never transforms back regardless of actual spell casts.

- Log message is incorrect when transforming from back face (Ulvenwald Primordials) to front face (Ulvenwald Mystics)
  - File: `mtg-engine/src/cards/isd/ulvenwald_mystics.rs` line 117
  - Oracle text says: `"transform this creature"` (the source is the current face, Ulvenwald Primordials)
  - Code does: `format!("Ulvenwald Mystics transforms into {}", name)` — hardcodes "Ulvenwald Mystics" as the source regardless of which face is currently active. When back→front occurs, `name = "Ulvenwald Mystics"` (set after the toggle), so the log reads `"Ulvenwald Mystics transforms into Ulvenwald Mystics"`. Should read `"Ulvenwald Primordials transforms into Ulvenwald Mystics"`. (This log error is currently masked by the engine bug above, because the back-to-front transform never fires in real gameplay.)

### Tricky interactions checked

- **"each upkeep" fires on both players' upkeep steps**: PASS — `triggers.rs` `GameEvent::StepStarted { step: Upkeep }` handler (lines 597–643) scans all battlefield permanents regardless of which player's turn it is; trigger fires for every upkeep step in the game.
- **Front-face condition "if no spells were cast last turn" (any player, total = 0)**: FAIL in practice due to engine bug — `spells_cast_last_turn` is always empty, so the sum is always 0 and Ulvenwald Mystics always transforms after turn 1. Card logic itself (sum across all players = 0) correctly matches the oracle's "no spells" wording.
- **Back-face condition "if a player cast two or more spells last turn" (single player ≥ 2, not total)**: FAIL in practice due to engine bug — `spells_cast_last_turn` is always empty, so `any(|&count| count >= 2)` is always false and Ulvenwald Primordials never transforms back. Card logic itself (`.any(|&count| count >= 2)` per-player check) correctly matches the oracle's "a player" wording.
- **First-turn guard (`!state.is_first_turn`)**: PASS — `is_first_turn` is set `true` at game start (`state.rs` line 218) and flipped to `false` at turn end (`engine.rs` line 2887). Front-face condition includes `&& !state.is_first_turn`, so Ulvenwald Mystics will not transform during turn 1's upkeep.
- **Upkeep trigger fires for transformed (back) face despite empty `back_face_data().triggered_abilities`**: PASS — `trigger_description` (`triggers.rs` lines 311–327) checks front-face `triggered_abilities` first; the front face declares `TriggerKind::Upkeep`, so a non-empty description is returned even when `is_transformed = true`. `on_upkeep` then reads `is_transformed` to choose the correct condition.
- **Regenerate ability available in response to the transform trigger**: PASS — `activated_abilities` gates on `o.is_transformed` (`ulvenwald_mystics.rs` lines 83–86). When the back-face upkeep trigger is on the stack (before resolution), `is_transformed` is still `true`, so the Regenerate ability is offered as a legal action, matching the ruling.
- **Regeneration shield persists through transformation (ruling: shield applies to Ulvenwald Mystics)**: PASS — `regeneration_shields` is stored on the `GameObject` identified by `ObjectId`. Transformation (`on_upkeep`) only flips `is_transformed` and updates `obj.name`; it does not touch `regeneration_shields`. The shield is only cleared on zone change (`state.rs` line 486, only when leaving the battlefield) or at the cleanup step (`engine.rs` line 3031, after all actions in the turn). So a shield activated while the creature is Ulvenwald Primordials remains on the same object after it becomes Ulvenwald Mystics.
- **P/T on transformed state (5/5 for Ulvenwald Primordials)**: PASS — `dynamic_pt` (`ulvenwald_mystics.rs` lines 74–80) returns `Some((5, 5))` when `is_transformed = true`. `effective_power` and `effective_toughness` in `state.rs` (lines 866–872, 910–916) prioritize `dynamic_pt` over `obj.power`, correctly yielding 5/5.
- **Source leaves battlefield between trigger and resolution**: PASS — `on_upkeep` (`ulvenwald_mystics.rs` lines 108–109) re-checks `zone == Battlefield` before acting; `resolve_next_trigger` for `UpkeepTrigger` (`triggers.rs` lines 954–959) also re-checks before calling the handler. If the permanent has left, neither executes.
- **"Transform" keyword word in Scryfall keywords field**: PASS — "Transform" is a keyword action, not a keyword ability. `keywords: vec![]` in `card_data()` is correct; the absence of "Transform" from the `keywords` vec is not an issue.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Front face transforms when no spells were cast last turn: `werewolf_cards.rs:551` (`ulvenwald_mystics_transforms_and_gains_regenerate`) — TESTED
- Front face does NOT transform when any spell was cast last turn: NOT TESTED for Ulvenwald Mystics (tested for Reckless Waif at `werewolf_cards.rs:61`, manually inserting into `spells_cast_last_turn`)
- Front face does NOT transform on the first turn: NOT TESTED for Ulvenwald Mystics (tested for Reckless Waif at `werewolf_cards.rs:47`)
- Back face transforms back when a player cast 2+ spells: NOT TESTED for Ulvenwald Mystics/Primordials
- Back face stays transformed when only 1 spell was cast: NOT TESTED for Ulvenwald Mystics/Primordials
- Regenerate ability available only on back face: `werewolf_cards.rs:551` — TESTED (front face has 0 abilities, back face has 1)
- Regenerate ability used to prevent destruction: NOT TESTED for Ulvenwald Primordials
- Regeneration shield persisting through transformation (ruling): NOT TESTED
- Engine actually increments `spells_cast_this_turn` when a spell is cast: NOT TESTED anywhere (no integration test exercises the real game loop and checks the werewolf conditions without manually setting `spells_cast_last_turn`)
- Engine transfers `spells_cast_this_turn` to `spells_cast_last_turn` at turn end: NOT TESTED
- Back face P/T is 5/5: `werewolf_cards.rs:565` (`assert_eq!(state.effective_power(mystics, &reg).unwrap(), 5)`) — TESTED
- Trigger fires on both players' upkeeps: NOT TESTED for Ulvenwald Mystics
- Source leaves battlefield before trigger resolves: NOT TESTED for Ulvenwald Mystics
- Log message when transforming back to front: NOT TESTED
