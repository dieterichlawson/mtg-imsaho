## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Back face oracle text**: At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Rogue Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- **Engine never populates `spells_cast_this_turn` or `spells_cast_last_turn`; both conditions are permanently broken**
  - Oracle text says (front face): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`
  - Oracle text says (back face): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`
  - Card does (front face check, `mtg-engine/src/cards/isd/reckless_waif.rs:12-14`): `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum(); if !is_transformed { total_spells_last_turn == 0 && !state.is_first_turn }`
  - Card does (back face check, `mtg-engine/src/cards/isd/reckless_waif.rs:16`): `state.spells_cast_last_turn.values().any(|&count| count >= 2)`
  - Engine does (`mtg-engine/src/engine.rs` CastSpell handler, lines 1479–1666): never increments `spells_cast_this_turn` when a spell is cast — zero matches for `spells_cast_this_turn` in the entire file.
  - Engine does (`mtg-engine/src/engine.rs` `advance_step`, lines 2882–2895): never transfers `spells_cast_this_turn` into `spells_cast_last_turn` at turn end — the only fields cleared are `is_first_turn` and `creature_died_this_turn`.
  - Net effect: `spells_cast_last_turn` is always an empty `HashMap`. Therefore `total_spells_last_turn` is always 0 → front face condition is permanently true after turn 1 (waif always transforms on every non-first-turn upkeep, even when spells were cast). Back face `any(|&count| count >= 2)` is always false → Merciless Predator never transforms back regardless of how many spells were cast.

### Tricky interactions checked

- **"each upkeep" fires on both players' upkeep steps**: PASS — `StepStarted { step: Upkeep }` in `triggers.rs` iterates all battlefield permanents regardless of whose turn it is; the trigger fires on every upkeep.
- **Front-face condition "if no spells were cast last turn" (sum across all players)**: FAIL — `spells_cast_last_turn` is never populated; `total_spells_last_turn` is always 0. Front face always evaluates to "no spells cast" even after a turn where multiple spells were cast.
- **Back-face condition "if a player cast two or more spells last turn" (any one player ≥ 2)**: FAIL — same root cause; `spells_cast_last_turn` is always empty, so `any(|&count| count >= 2)` is always false. Merciless Predator is permanently stuck and never transforms back.
- **First-turn protection (`!state.is_first_turn`)**: PASS — `is_first_turn` is correctly set to `true` at game start and flipped to `false` at the end of player 1's first turn. Front face will not transform during player 1's first upkeep.
- **Transform toggle logic (obj.is_transformed flip + name update)**: PASS — `on_upkeep` correctly flips `is_transformed` and updates `obj.name` for both directions.
- **Back face P/T via `dynamic_pt`**: PASS — `dynamic_pt` returns `Some((3, 2))` when `is_transformed` is true; `effective_power`/`effective_toughness` in `state.rs` use this override correctly, yielding 3/2 for Merciless Predator.
- **Back face upkeep trigger dispatch (no trigger in `back_face_data`)**: PASS — `trigger_description` in `triggers.rs` checks the front face first (lines 314–326); since the front face has `TriggerKind::Upkeep` registered, it returns a non-empty description for both front and back face instances, so the upkeep trigger is correctly collected even when the permanent is transformed.
- **"should_transform" reads current face correctly**: PASS — `werewolf_should_transform` reads `obj.is_transformed` to choose which condition to evaluate, correctly separating front-face logic from back-face logic.
- **Log message accuracy on transform-back**: The log at `reckless_waif.rs:86` always says `"Reckless Waif transforms into {}"` even when the back face (Merciless Predator) transforms to the front face. This produces "Reckless Waif transforms into Reckless Waif" which is cosmetically wrong, but does not affect game mechanics.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Front-face transforms when no spells cast last turn (card logic in isolation): `werewolf_cards.rs:28` — TESTED (manually clears `spells_cast_last_turn`)
- Front-face does NOT transform on the first turn: `werewolf_cards.rs:48` — TESTED
- Front-face does NOT transform when any spell was cast last turn (card logic in isolation): `werewolf_cards.rs:61` — TESTED (manually inserts into `spells_cast_last_turn`)
- Back-face transforms back when 2+ spells cast by a player (card logic in isolation): `werewolf_cards.rs:74` — TESTED (manually inserts into `spells_cast_last_turn`)
- Back-face stays transformed when only 1 spell cast: `werewolf_cards.rs:663` — TESTED (manually inserts into `spells_cast_last_turn`)
- "each upkeep" fires on both players' upkeep (both upkeeps fire trigger): NOT TESTED
- Engine actually increments `spells_cast_this_turn` when a spell is cast via the game loop: NOT TESTED
- Engine saves `spells_cast_this_turn` to `spells_cast_last_turn` at turn end: NOT TESTED
- Back-face P/T is 3/2 after transform: `werewolf_cards.rs:43` — TESTED
- Multiple werewolves transform simultaneously: `werewolf_cards.rs:625` — TESTED
- Transformed werewolf loses Human subtype: `werewolf_cards.rs:600` — TESTED
