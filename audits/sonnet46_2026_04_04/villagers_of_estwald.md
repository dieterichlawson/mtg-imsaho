## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of each upkeep, if no spells were cast last turn, transform this creature. // At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- **Engine never populates `spells_cast_last_turn` in real games** (`mtg-engine/src/engine.rs`, `mtg-engine/src/state.rs`)
  - Oracle text says (front): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`
  - Oracle text says (back): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`
  - Code does: `state.spells_cast_last_turn` is declared in `state.rs` (line 131) and initialized to an empty `HashMap::new()` (line 231), but is **never populated during play**. The `CastSpell` action handler in `engine.rs` pushes a `GameEvent::SpellCast` but does not increment `state.spells_cast_this_turn`. The `advance_step` function (engine.rs lines 2867–2904) handles turn transitions but never copies `spells_cast_this_turn` into `spells_cast_last_turn`, nor clears `spells_cast_this_turn`. Confirmed by exhaustive grep across all `.rs` files outside state initialization: no call site ever writes to `spells_cast_this_turn` or `spells_cast_last_turn` during gameplay. Practical effect in a real game:
    - `total_spells_last_turn` (sum over empty map) is always 0 → front-face condition `total_spells_last_turn == 0 && !state.is_first_turn` is always true after turn 1 → Villagers always transforms at every upkeep regardless of whether spells were cast.
    - `state.spells_cast_last_turn.values().any(|&count| count >= 2)` over an empty map is always false → Howlpack never transforms back, even after 2+ spells were cast.

- **Log message is wrong when Howlpack transforms back to Villagers** (`mtg-engine/src/cards/isd/villagers_of_estwald.rs`, line 88)
  - Oracle text implies: a transform from Howlpack of Estwald back to Villagers of Estwald.
  - Code does: `format!("Villagers of Estwald transforms into {}", name)` where `name` is "Villagers of Estwald" (because `obj.is_transformed` was just set to `false`). The log reads "Villagers of Estwald transforms into Villagers of Estwald" when the card should be identified as Howlpack at the point the log is written. The correct log would be "Howlpack of Estwald transforms into Villagers of Estwald".

### Tricky interactions checked

- **"each upkeep" (fires on all players' upkeeps, not just controller's)**: PASS. The `StepStarted { step: Upkeep }` dispatch in `triggers.rs` (lines 605–640) iterates all permanents on the battlefield regardless of whose turn it is and fires upkeep triggers for each.
- **"if no spells were cast last turn" — zero total vs. zero per player**: PASS (logic). The front-face check uses `state.spells_cast_last_turn.values().sum()`, which counts total spells across all players. This correctly reflects the "no spells were cast last turn" wording. The underlying data is broken (never populated), but the logic over the data is correct.
- **"a player cast two or more spells" — single player must have 2+, not 2 combined**: PASS (logic). The back-face check uses `.any(|&count| count >= 2)`, which checks if any individual player's count ≥ 2, not the total. This correctly reflects "a player cast two or more" (i.e., one player cast ≥ 2, not one player cast 1 and another cast 1). Again, underlying data broken.
- **First turn guard (`!state.is_first_turn`)**: PASS. On turn 1 there is no "last turn," so the front-face transform condition is correctly blocked. `is_first_turn` is set to `false` after the first turn advances (engine.rs line 2887).
- **Back face upkeep trigger fires despite `triggered_abilities: vec![]` on back face**: PASS. `trigger_description` in `triggers.rs` (lines 311–327) first checks front-face triggered abilities and finds `TriggerKind::Upkeep` there (returning description "transform"). This causes the trigger to fire when the card is transformed as well, because the trigger-collection loop only requires a non-empty description.
- **Zone check in `on_upkeep`**: PASS. The handler guards against the card not being on the battlefield: `if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) { return; }`.
- **P/T for both faces**: PASS. Front face has static `power: Some(2), toughness: Some(3)`. Back face relies on `dynamic_pt` returning `Some((4, 6))` when `is_transformed == true`. Tests confirm this works (`villagers_of_estwald_transforms_to_large_body`).
- **Subtype correctness (Human on front, not back)**: PASS. Front face has `subtypes: vec!["Human".into(), "Werewolf".into()]`; back face has `subtypes: vec!["Werewolf".into()]`. No Human subtype on back face per oracle.
- **Upkeep trigger dispatch when transformed — trigger_description returns front face description**: PASS (functionally, despite being architecturally fragile). Trigger fires and `on_upkeep` dispatches on `is_transformed` to check the correct condition for each face.

### Test coverage

- **Front face transforms when no spells cast last turn**: `werewolf_cards.rs:156` (`villagers_of_estwald_transforms_to_large_body`) — tested manually by leaving `spells_cast_last_turn` empty. This test **bypasses the engine bug** by not populating spell counts through the game loop.
- **Front face does NOT transform when spells were cast last turn**: NOT TESTED for Villagers specifically (only tested for Reckless Waif at `werewolf_cards.rs:61`).
- **Back face transforms back when 2+ spells cast by a player**: NOT TESTED for Villagers/Howlpack specifically (only tested for Reckless Waif at `werewolf_cards.rs:74`).
- **Back face stays if only 1 spell cast**: NOT TESTED for Villagers/Howlpack specifically (only via `werewolf_cards.rs:663`).
- **Does not transform on first turn**: NOT TESTED for Villagers specifically (only tested for Reckless Waif at `werewolf_cards.rs:47` and Mayor at `werewolf_cards.rs:331`).
- **Engine correctly tracks spells cast per player between turns (end-to-end)**: NOT TESTED. No integration test exercises the full game loop and verifies that casting a spell on turn N causes the werewolf NOT to transform on turn N+1's upkeep. All werewolf tests set `spells_cast_last_turn` by hand, working around the engine bug.
- **Log message when transforming back (Howlpack → Villagers)**: NOT TESTED.
- **Fires on opponent's upkeep**: NOT TESTED for Villagers specifically.
