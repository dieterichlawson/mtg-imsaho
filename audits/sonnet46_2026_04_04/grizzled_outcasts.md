## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of each upkeep, if no spells were cast last turn, transform this creature. // At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- Engine never updates `spells_cast_this_turn` or `spells_cast_last_turn`; both transform conditions are always wrong
  - File: `mtg-engine/src/engine.rs` lines 1657–1665 (spell cast) and 2882–2895 (turn transition)
  - Oracle text says (front face): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`
  - Oracle text says (back face): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`
  - Code does: `spells_cast_this_turn` is initialized to `HashMap::new()` in `state.rs` line 230 and never incremented anywhere when a spell is cast. `spells_cast_last_turn` is initialized to `HashMap::new()` in `state.rs` line 231 and never populated from `spells_cast_this_turn` at any turn transition. The cleanup step (engine.rs ~3006–3060) and turn-end transition (engine.rs ~2882–2895) both omit any update to these fields. As a result, `spells_cast_last_turn` is permanently an empty map in real gameplay.
  - Consequence for front face: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` (grizzled_outcasts.rs line 12) always equals 0, so `total_spells_last_turn == 0 && !state.is_first_turn` (line 14) is always true after turn 1 — Grizzled Outcasts **always transforms** even when spells were cast last turn.
  - Consequence for back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 16) is always false — Krallenhorde Wantons **never transforms back** even when two or more spells were cast last turn.

- Log message is incorrect when transforming back to front face
  - File: `mtg-engine/src/cards/isd/grizzled_outcasts.rs` line 87–88
  - Code does: `format!("Grizzled Outcasts transforms into {}", name)` where `name` is computed after flipping `is_transformed`. When transforming from Krallenhorde Wantons → Grizzled Outcasts, `name = "Grizzled Outcasts"` and the log reads `"Grizzled Outcasts transforms into Grizzled Outcasts"`.
  - The log should read `"Krallenhorde Wantons transforms into Grizzled Outcasts"`. The source name is hardcoded as `"Grizzled Outcasts"` regardless of which face was active before the flip.

### Tricky interactions checked

- **Front-face upkeep trigger fires when card is transformed (back face)**: PASS. `trigger_description` (triggers.rs line 311–327) checks front-face `triggered_abilities` first; the front face declares `TriggerKind::Upkeep`, so the trigger fires on both faces. `on_upkeep` then checks `is_transformed` via `werewolf_should_transform` to determine direction. No trigger is missed.
- **"if no spells were cast last turn" condition (any player, not just active)**: PASS in card logic. `total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` sums across all players, correctly matching "no spells were cast" by anyone. FAIL in practice because `spells_cast_last_turn` is never populated (see above).
- **"if a player cast two or more spells last turn" condition (single player, ≥2)**: PASS in card logic. `state.spells_cast_last_turn.values().any(|&count| count >= 2)` correctly checks for any single player with count ≥ 2, not total. FAIL in practice because the map is never populated.
- **First-turn guard (`!state.is_first_turn`)**: PASS. Front-face condition is `total_spells_last_turn == 0 && !state.is_first_turn`; the `is_first_turn` flag is correctly set to `true` on startup (state.rs line 218) and cleared when the turn ends (engine.rs line 2887).
- **P/T on transformed state**: PASS. `dynamic_pt` (grizzled_outcasts.rs lines 70–76) returns `Some((7, 7))` when `is_transformed = true`, giving Krallenhorde Wantons the correct 7/7 stats.
- **Transform flag and name update**: PASS. `obj.is_transformed = !obj.is_transformed` correctly toggles the flag; the name is then derived from the new flag value, so name and flag stay in sync.
- **Back face `triggered_abilities: vec![]`**: PASS for trigger dispatch (front-face trigger covers both). However, the back face having no declared upkeep trigger means `trigger_description` never returns the back-face description when checking transformed permanents — the front-face description "transform" is used for both directions. This is cosmetic only.
- **Upkeep trigger fires on EACH upkeep (both players' turns)**: PASS. `trigger_description` in triggers.rs lines 597–643 scans ALL battlefield permanents on every `StepStarted { step: Upkeep }` event, not just the active player's turn.
- **Source leaves battlefield between trigger and resolution**: PASS. `on_upkeep` at grizzled_outcasts.rs lines 79–80 re-checks `zone == Battlefield` before acting; the UpkeepTrigger resolution in triggers.rs lines 954–959 also re-checks before calling the handler.
- **"Transform" keyword in card data**: PASS (not an issue). "Transform" is listed as a keyword in Scryfall's Keywords field but it is a keyword action, not a keyword ability. The `keywords: vec![]` in card_data is correct.

### Test coverage

For each ruling and tricky interaction:
- Front face transforms when no spells cast: `werewolf_cards.rs:221–233` (`grizzled_outcasts_transforms_to_7_7`) — TESTED (works only because `spells_cast_last_turn` is permanently empty)
- Front face does NOT transform when spells were cast: NOT TESTED for Grizzled Outcasts specifically (tested for Reckless Waif via `human_side_stays_if_any_spell_cast`, manually injecting `spells_cast_last_turn`)
- Back face transforms back when 2+ spells cast: NOT TESTED for Grizzled Outcasts/Krallenhorde Wantons
- Back face does NOT transform back when <2 spells cast: NOT TESTED for Grizzled Outcasts/Krallenhorde Wantons
- First-turn guard: `werewolf_cards.rs:47–58` (`reckless_waif_stays_human_on_first_turn`) for Reckless Waif; NOT TESTED for Grizzled Outcasts
- Engine tracks `spells_cast_this_turn` and rotates to `spells_cast_last_turn`: NOT TESTED anywhere (no integration test casts a spell and then checks werewolf transform conditions without manually setting `spells_cast_last_turn`)
- Trigger fires on both players' upkeeps: NOT TESTED for Grizzled Outcasts
- Source leaves battlefield before trigger resolves: NOT TESTED for Grizzled Outcasts
- Log message when transforming back: NOT TESTED
