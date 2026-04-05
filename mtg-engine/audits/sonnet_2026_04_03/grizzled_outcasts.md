## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: 
- Front face: At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- Back face: At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Werewolf (front) / Creature — Werewolf (back)
**Status**: ISSUE

### Code issues

- Log message always uses front face name: `grizzled_outcasts.rs:88`
  - Oracle text says: Back face has its own transform ability with oracle text "At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."
  - Code does: Always logs `"Grizzled Outcasts transforms into {}"` even when transforming from back face to front face, should log current face name transforming to new name

- Back face trigger registration missing: `grizzled_outcasts.rs:62`
  - Oracle text says: `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."` (back face has upkeep trigger)
  - Code does: Declares `triggered_abilities: vec![]` (empty) for back face, relies on front face trigger instead

### Tricky interactions checked

- First turn handling: PASS - code includes `&& !state.is_first_turn` to prevent transformation on first turn
- Spell count tracking for front face (0 spells): PASS - checks `total_spells_last_turn == 0` correctly
- Spell count tracking for back face (2+ spells by any player): PASS - checks `state.spells_cast_last_turn.values().any(|&count| count >= 2)` correctly
- Transform trigger collection: PASS - trigger system finds front face trigger regardless of transformed state via `trigger_description()` function
- Transform condition evaluation: PASS - `werewolf_should_transform()` correctly evaluates different conditions based on `is_transformed` state
- Dynamic P/T updates: PASS - `dynamic_pt()` returns `(7,7)` when transformed, front face uses base stats
- Name updates on transform: PASS - correctly updates object name to "Krallenhorde Wantons"/"Grizzled Outcasts"

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Front to back transformation: `werewolf_cards.rs:220-233` (grizzled_outcasts_transforms_to_7_7)
- Back to front transformation (2+ spells cast): NOT TESTED (only generic werewolf tests cover this pattern)
- No transformation on first turn: `werewolf_cards.rs:331-342` (mayor_of_avabruck_does_not_transform_on_first_turn - covers werewolf pattern)
- No transformation when wrong spell count: `werewolf_cards.rs:679-691` (generic werewolf test covers this pattern)
- Multiple werewolves transforming together: `werewolf_cards.rs:624-638` (generic test including Grizzled Outcasts)