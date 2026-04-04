## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of each upkeep, if no spells were cast last turn, transform this creature. // At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Warrior Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- **Engine never tracks spells cast per turn; `spells_cast_last_turn` is always empty in real gameplay** (`mtg-engine/src/engine.rs`, `mtg-engine/src/state.rs`)
  - Oracle text says: `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."` / `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`
  - Code does: `state.spells_cast_this_turn` is defined in `state.rs:127` as `HashMap<PlayerId, u32>` and initialized to `HashMap::new()` (`state.rs:230`) but is **never incremented anywhere in the engine**. When a spell is cast (`engine.rs:1657`), only a `GameEvent::SpellCast` is pushed; `spells_cast_this_turn` is not updated. Additionally, `spells_cast_last_turn` is never assigned from `spells_cast_this_turn` at turn boundaries (the end-of-turn transition at `engine.rs:2882–2895` does not perform this copy). As a result, in real gameplay `spells_cast_last_turn` is always an empty map. The front-face condition `total_spells_last_turn == 0 && !state.is_first_turn` (`tormented_pariah.rs:14`) is therefore always true after the first turn, causing Tormented Pariah to transform unconditionally at every upkeep regardless of spells cast. The back-face condition `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (`tormented_pariah.rs:16`) is always false, so Rampaging Werewolf can never transform back.

- **Log message incorrectly names source when Rampaging Werewolf transforms back** (`mtg-engine/src/cards/isd/tormented_pariah.rs:87–88`)
  - Oracle text says: the creature is named "Rampaging Werewolf" while in its back-face form.
  - Code does: `format!("Tormented Pariah transforms into {}", name)` — when the card is on its back face and transforms back, `name` is `"Tormented Pariah"`, yielding the log string `"Tormented Pariah transforms into Tormented Pariah"`. The card at that moment is named "Rampaging Werewolf", so the log is doubly wrong: the source name is wrong and the message implies no change.

### Tricky interactions checked

- **"Each upkeep" (fires on every player's upkeep, not just controller's)**: PASS — `triggers.rs:605` collects all battlefield permanents regardless of controller when `StepStarted { step: Upkeep }` fires; the card's upkeep trigger will be collected on every player's upkeep.
- **Back-face upkeep trigger collection (Rampaging Werewolf)**: PASS — `trigger_description` (`triggers.rs:311–327`) checks front-face `triggered_abilities` first; since Tormented Pariah's front face registers `TriggerKind::Upkeep`, the description `"transform"` is returned for both the front and back face, so the upkeep trigger is collected in both states.
- **Back-face condition evaluated correctly (when data is present)**: PASS — `werewolf_should_transform` branches on `is_transformed` (`tormented_pariah.rs:13–17`), applying the back-face "2+ spells" condition when transformed. Correct in isolation, but broken in practice due to the engine bug above.
- **Front-face "no spells cast last turn" condition**: FAIL — Due to engine never populating `spells_cast_last_turn`, this condition is always satisfied (empty map sums to 0), causing unconditional transformation.
- **"A player cast two or more spells" (any player, not just a specific one)**: PASS — code uses `.values().any(|&count| count >= 2)` which correctly checks any player. But the data is never populated.
- **First-turn guard**: PASS — `!state.is_first_turn` (`tormented_pariah.rs:14`) prevents transformation on the very first turn of the game, matching the MTG rule that werewolves don't transform if the game just started.
- **Object-still-on-battlefield check in `on_upkeep`**: PASS — `tormented_pariah.rs:79` returns early if the object is not on the battlefield.
- **P/T values (front face 3/2, back face 6/4)**: PASS — `dynamic_pt` (`tormented_pariah.rs:70–76`) returns `Some((6, 4))` when transformed, and the base `power: Some(3), toughness: Some(2)` covers the front face.
- **Mana cost {3}{R}**: PASS — `cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)]))` matches oracle.
- **Subtypes (Human Warrior Werewolf / Werewolf)**: PASS — front face has `vec!["Human".into(), "Warrior".into(), "Werewolf".into()]`, back face has `vec!["Werewolf".into()]`.
- **Back-face has no mana cost**: PASS — `cost: None` on back-face data.
- **`SpellCast` event only fires SpellCastWatch for instants/sorceries** (`triggers.rs:644–675`): Noted — the werewolf spell-tracking fields are supposed to count ALL spells, not just instants/sorceries. However, since the tracking is never done by the engine at all, this is subsumed by the main engine bug.

### Test coverage

- Front-face transforms to Rampaging Werewolf (6/4) when no spells cast: `werewolf_cards.rs:203` — TESTED (but bypasses real engine by leaving `spells_cast_last_turn` empty by default)
- Front-face does NOT transform when spells were cast last turn: NOT TESTED for Tormented Pariah specifically (tested only for Reckless Waif at `werewolf_cards.rs:60`)
- Back-face transforms back when a player cast 2+ spells: NOT TESTED for Tormented Pariah/Rampaging Werewolf
- Back-face does not transform back when only 1 spell cast: NOT TESTED for Tormented Pariah/Rampaging Werewolf
- First-turn no-transform guard: NOT TESTED for Tormented Pariah specifically
- Engine actually increments `spells_cast_this_turn` when a spell is cast: NOT TESTED (no integration test verifies that real spell-casting updates the count)
- Engine copies `spells_cast_this_turn` → `spells_cast_last_turn` at turn boundary: NOT TESTED
- "Each upkeep" fires on both players' upkeep steps: NOT TESTED for Tormented Pariah
- Log message accuracy when transforming back to human form: NOT TESTED
