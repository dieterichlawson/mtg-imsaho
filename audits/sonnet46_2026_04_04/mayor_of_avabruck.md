## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Other Human creatures you control get +1/+1.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
— Back face (Howlpack Alpha) —
Each other creature you control that's a Werewolf or a Wolf gets +1/+1.
At the beginning of your end step, create a 2/2 green Wolf creature token.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Advisor Werewolf / Creature — Werewolf
**Status**: ISSUE

### Code issues

- **Engine never populates `spells_cast_last_turn`** — both transform conditions are permanently broken in actual gameplay.
  - `mtg-engine/src/state.rs:127,131`: `spells_cast_this_turn` and `spells_cast_last_turn` are defined as `HashMap<PlayerId, u32>`, initialized empty, and **never written to** anywhere in the engine source (confirmed by exhaustive search of `mtg-engine/src/`).
  - `mtg-engine/src/engine.rs` `CastSpell` handler (lines 1479–1666): handles spell casting in full but contains no increment of `spells_cast_this_turn`.
  - `mtg-engine/src/engine.rs` `advance_step` (lines 2867–2904): handles end-of-turn transition (sets new active player, increments turn_number, clears `creature_died_this_turn`) but never transfers `spells_cast_this_turn` → `spells_cast_last_turn`.
  - Consequence for front face: `total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` is always 0; condition `total_spells_last_turn == 0 && !state.is_first_turn` is always true after turn 1. Mayor **always** transforms every upkeep regardless of whether spells were cast.
    - Oracle text says: `"if no spells were cast last turn, transform this creature"`
    - Code does: evaluates `spells_cast_last_turn` which is always empty → condition always true → always transforms
  - Consequence for back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` is always false. Howlpack Alpha **never** transforms back regardless of how many spells were cast.
    - Oracle text says: `"if a player cast two or more spells last turn, transform this creature"`
    - Code does: evaluates `spells_cast_last_turn` which is always empty → condition always false → never transforms back
  - All tests bypass this by directly inserting into `spells_cast_last_turn` (e.g., `state.spells_cast_last_turn.insert(P0, 2)`), so unit tests pass even though the engine never populates the field.

- **Log message hardcodes wrong source name when transforming back** — `mtg-engine/src/cards/isd/mayor_of_avabruck.rs:119`
  - Oracle text says: (transform event — Howlpack Alpha becomes Mayor of Avabruck)
  - Code does: `format!("Mayor of Avabruck transforms into {}", name)` — `name` is `"Mayor of Avabruck"` when transforming back, producing log message `"Mayor of Avabruck transforms into Mayor of Avabruck"`. At that point the permanent is named "Howlpack Alpha". The format string should use the pre-flip name, not a hardcoded string. The resulting log message inaccurately says the card transformed into itself.

### Tricky interactions checked

- **"each upkeep" fires on every player's upkeep, not just controller's**: pass — `triggers.rs` dispatch collects Upkeep triggers for all battlefield permanents on any `StepStarted { step: Upkeep }` event with no player filter; `on_upkeep` itself has no player restriction.
- **"if no spells were cast last turn" counts spells by any player**: pass in card logic — `total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` sums across all players; fail at engine level because the map is never populated (see issue above).
- **"if a player cast two or more spells last turn" requires a single player with ≥2 casts (not total ≥2)**: pass in card logic — `state.spells_cast_last_turn.values().any(|&count| count >= 2)` correctly requires one player's count ≥2; fail at engine level for same reason.
- **First-turn protection**: pass — `!state.is_first_turn` guard in front-face condition prevents transform on turn 1 upkeep.
- **Back face continuous effect switches in correctly**: pass — `continuous_pt_mods` in `state.rs:746` uses `behavior.back_face_data().map(|d| d.continuous_effects)` when `source.is_transformed`, so the Human buff is off and the Wolf/Werewolf buff is on when transformed.
- **"other" excludes the source itself**: pass — `EffectScope::GlobalOther` in `effect_applies_to` (`state.rs:719–721`) checks `creature_id != source_id`.
- **"as long as" continuous re-evaluation**: pass — effects are re-evaluated on each call to `effective_power`/`effective_toughness`, not snapshot at ETB.
- **Werewolf+Wolf creature gets only +1/+1 (2025-01-24 ruling)**: pass — back face uses `CreatureFilter::Or([Werewolf, Wolf])` inside a single `ModifyPT` entry, so a creature matching both subtypes still receives exactly +1/+1.
- **Wolf token subtype check via object-level subtypes**: pass — `matches_filter` for `HasSubtype` falls through to `creature.subtypes.iter().any(|s| s == subtype)` which reads the token's object-level subtypes (CardId 0 tokens have no registry entry).
- **"At the beginning of your end step" fires only on controller's turn**: pass — `on_end_step` checks `state.active_player != controller` and returns early if mismatch.
- **Wolf token created with correct attributes (2/2, green, Creature, Wolf subtype)**: pass — `create_token_with_subtypes("Wolf", controller, 2, 2, vec![Color::Green], vec![CardType::Creature], vec![], vec!["Wolf".into()])`.
- **Dynamic P/T (3/3 on back face) applied via `dynamic_pt`**: pass — `effective_power` calls `behavior.dynamic_pt(self, id)` which returns `Some((3, 3))` when `is_transformed` is true.
- **Subtype filter for transformed DFCs checks back face subtypes only**: pass — `matches_filter` `HasSubtype` branch checks `back_face_data().subtypes` when `is_transformed`, not front face.
- **Trigger description for Howlpack Alpha's upkeep shows front-face description**: the `trigger_description` function returns the front face's "transform" description even for the transformed state because it finds a front-face `Upkeep` trigger first and returns early; the stack display shows the wrong description. Does not affect behavioral correctness (the `on_upkeep` handler correctly evaluates `should_transform` based on face) but is cosmetically misleading.

### Test coverage

- "if no spells were cast last turn, transform" — front face: `werewolf_cards.rs:28` (reckless_waif, but logic shared), `werewolf_cards.rs:332` (mayor-specific); NOT TESTED through full engine game loop (manual `spells_cast_last_turn.insert` bypass only).
- "if any spell was cast last turn, do NOT transform" — front face: `werewolf_cards.rs:61` (manual insert); NOT TESTED through full engine game loop.
- "if a player cast 2+ spells last turn, transform back" — back face: `werewolf_cards.rs:74` (manual insert); NOT TESTED through full engine game loop.
- `spells_cast_last_turn` populated by engine during gameplay: NOT TESTED — no integration test casts spells and then checks werewolf transform state.
- Human buff (+1/+1) from front face: `werewolf_cards.rs:238` — TESTED.
- Mayor doesn't buff itself: `werewolf_cards.rs:248` — TESTED.
- Back face buffs Werewolves/Wolves: `werewolf_cards.rs:252` — TESTED.
- Werewolf+Wolf only gets +1/+1 (ruling 2025-01-24): `werewolf_cards.rs:345` — TESTED.
- Wolf token creation on end step: `werewolf_cards.rs:273` — TESTED.
- No token on front face: `werewolf_cards.rs:294` — TESTED.
- No token on opponent's end step: `werewolf_cards.rs:310` — TESTED.
- First-turn no-transform protection: `werewolf_cards.rs:331` — TESTED.
- Log message "Mayor of Avabruck transforms into Mayor of Avabruck" bug: NOT TESTED.
