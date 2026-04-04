## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Back face oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.) / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Werewolf (front) / Creature — Werewolf (back)
**Status**: ISSUE

### Code issues

- **Engine never increments `spells_cast_this_turn`; `spells_cast_last_turn` is always empty, making both transform conditions permanently wrong**
  - `mtg-engine/src/engine.rs` lines 1479–1666: The `Action::CastSpell` handler fires `GameEvent::SpellCast` but never writes to `state.spells_cast_this_turn`. Searched entire codebase: `spells_cast_this_turn` appears only in `state.rs` (declaration at line 127, initialization at line 230) and nowhere else.
  - `mtg-engine/src/engine.rs` lines 2880–2903 (turn transition in `advance_step`) and lines 3006–3061 (`Step::Cleanup` in `perform_turn_based_actions`): Neither location copies `spells_cast_this_turn` to `spells_cast_last_turn` or clears `spells_cast_this_turn`. Both fields remain empty `{}` throughout the entire game.
  - **Front face consequence**: `werewolf_should_transform` checks `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` (line 12 of `gatstaf_shepherd.rs`), then `total_spells_last_turn == 0 && !state.is_first_turn` (line 14). Since `spells_cast_last_turn` is always empty, the sum is always 0, so this condition is always `true` after the first turn. Gatstaf Shepherd transforms at every upkeep regardless of spells cast.
  - **Back face consequence**: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 16 of `gatstaf_shepherd.rs`) on an empty map always returns `false`. Gatstaf Howler never transforms back regardless of how many spells were cast.
  - Oracle text says (front): `"if no spells were cast last turn"` — should only trigger when the total spell count for the turn was zero.
  - Oracle text says (back): `"if a player cast two or more spells last turn"` — should trigger when any one player cast ≥ 2 spells last turn.
  - Code does: front face transforms unconditionally every upkeep (after turn 1); back face never transforms back.

- **Log message names the wrong source card when transforming back to front face**
  - `mtg-engine/src/cards/isd/gatstaf_shepherd.rs` lines 87–89:
    ```rust
    state.log(crate::state::LogLevel::Event,
        format!("Gatstaf Shepherd transforms into {}", name));
    ```
  - The format string hardcodes `"Gatstaf Shepherd"` as the subject of transformation. When the card is on its back face (Gatstaf Howler) and transforms back, `is_transformed` is set to `false`, `name` becomes `"Gatstaf Shepherd"`, and the log reads `"Gatstaf Shepherd transforms into Gatstaf Shepherd"` — which is wrong; it should say `"Gatstaf Howler transforms into Gatstaf Shepherd"`.
  - Oracle text does not specify log message content, but this is an implementation inaccuracy that misrepresents the game event to observers.

### Tricky interactions checked

- **First-turn guard (`is_first_turn`)**: PASS — code correctly adds `&& !state.is_first_turn` (line 14) preventing transform on turn 1's upkeep when `spells_cast_last_turn` would also be empty for a legitimate reason.
- **Front-face spell count logic (sum across all players)**: PASS (logic is correct) — uses `values().sum()` to get total spells by all players; oracle "no spells" means zero total, so this is correct. Would work if the data were ever populated.
- **Back-face spell count logic (any player cast 2+)**: PASS (logic is correct) — uses `.any(|&count| count >= 2)`; oracle "a player cast two or more" means any single player ≥ 2, not combined total. Logic is correct. Would work if data were populated.
- **Engine spell count never populated (critical)**: FAIL — `spells_cast_this_turn` is declared in `state.rs:127` but never incremented in `engine.rs` during `CastSpell` handling (confirmed by exhaustive search). `spells_cast_last_turn` is never updated at turn boundaries. Both conditions are therefore permanently wrong in real gameplay.
- **Trigger fires for back face (empty `triggered_abilities`)**: PASS — back face `triggered_abilities: vec![]` (line 62), but `trigger_description()` in `triggers.rs:311–327` checks front face triggers first, finds `TriggerKind::Upkeep`, and returns description `"transform"`. The `UpkeepTrigger` is created and `on_upkeep` is called regardless of transform state.
- **`has_keyword` for Intimidate on back face**: PASS — `state.rs:1006–1011` checks `back_face_data().keywords` when `is_transformed == true`; back face correctly declares `keywords: vec![Keyword::Intimidate]` (line 58).
- **Subtype changes on transform (Human dropped, Werewolf kept)**: PASS — `state.rs:654–672` `HasSubtype` filter uses back face subtypes when `is_transformed == true`; back face declares `subtypes: vec!["Werewolf".into()]` only (line 54), no Human.
- **P/T 3/3 on back face via `dynamic_pt`**: PASS — `dynamic_pt` (lines 70–76) returns `Some((3, 3))` when `is_transformed == true`, matching back face 3/3.
- **Front face P/T 2/2**: PASS — declared `power: Some(2), toughness: Some(2)` (lines 32–33).
- **Intervening-if clause ("if no spells were cast last turn")**: PASS — the condition is checked at resolution time in `on_upkeep` via `should_transform`. No separate check at trigger-collection time; both times the same condition is evaluated.
- **Log message on forward transform**: FAIL (cosmetic) — format string always reads `"Gatstaf Shepherd transforms into {name}"`, which is correct for the forward transform but wrong for the reverse.

### Test coverage

For each ruling and tricky interaction:

- Front face transforms when no spells cast: `tests/werewolf_cards.rs` — covered by `gatstaf_shepherd_transforms_and_gains_intimidate` (line 97). Tests set `spells_cast_last_turn` to empty (default) and verify transform. TESTED (but only by manually leaving the map empty; does not test that the engine populates it correctly).
- Front face does NOT transform when spells were cast: NOT DIRECTLY TESTED for Gatstaf Shepherd (tested for Reckless Waif at line 61 with manual `spells_cast_last_turn.insert`).
- First-turn no-transform guard: NOT TESTED for Gatstaf Shepherd (tested for Reckless Waif at line 47 and Mayor at line 331).
- Back face transforms back when ≥2 spells cast: `tests/werewolf_cards.rs` — covered by `gatstaf_shepherd_loses_intimidate_on_transform_back` (line 113). Manually sets `spells_cast_last_turn.insert(P1, 2)`. TESTED.
- Back face does NOT transform back with only 1 spell cast: covered indirectly by `werewolf_side_stays_if_one_spell_cast` (line 662, tests Reckless Waif).
- Engine actually populates `spells_cast_this_turn` when a spell is cast: NOT TESTED anywhere.
- Engine transfers `spells_cast_this_turn` to `spells_cast_last_turn` at turn boundary: NOT TESTED anywhere.
- Intimidate present on back face: `tests/werewolf_cards.rs:109` — `gatstaf_shepherd_transforms_and_gains_intimidate`. TESTED.
- Intimidate absent on front face: `tests/werewolf_cards.rs:128` — `gatstaf_shepherd_loses_intimidate_on_transform_back`. TESTED.
- Subtype Human dropped on transform: `tests/werewolf_cards.rs:599` — `transformed_werewolf_has_werewolf_subtype_not_human` (tests Reckless Waif). Not tested for Gatstaf Shepherd directly, but same engine path. NOT TESTED for this card specifically.
- Multiple werewolves transform together: `tests/werewolf_cards.rs:625` — `multiple_werewolves_transform_on_same_upkeep`. TESTED (includes Gatstaf Shepherd).
- Multiple werewolves transform back together: `tests/werewolf_cards.rs:641` — `multiple_werewolves_transform_back_together`. TESTED (includes Gatstaf Shepherd).
- Log message accuracy when transforming back: NOT TESTED.
