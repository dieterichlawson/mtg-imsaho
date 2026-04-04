## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
--- Back Face (Terror of Kruin Pass) ---
Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Rogue Werewolf / Creature — Werewolf (back)
**Status**: ISSUE

### Code issues

- **Engine never increments `spells_cast_this_turn` when a spell is cast, and never saves it to `spells_cast_last_turn` at turn end** (`mtg-engine/src/engine.rs` CastSpell handler ~line 1657; `advance_step` ~line 2882)
  - Oracle text says: `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."` and `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`
  - Code does: The `CastSpell` action handler pushes `GameEvent::SpellCast` (line 1657) but never executes `state.spells_cast_this_turn.entry(player).and_modify(|e| *e += 1).or_insert(1)`. The `advance_step` end-of-turn branch (line 2882) never copies `spells_cast_this_turn` into `spells_cast_last_turn` and never clears `spells_cast_this_turn`. Both maps remain empty `HashMap::new()` throughout actual gameplay. Consequence: `total_spells_last_turn` is always `0` (sum of an empty map), so the front-face condition `total_spells_last_turn == 0 && !state.is_first_turn` is always `true` after turn 1 — Kruin Outlaw always transforms regardless of spells cast. The back-face condition `state.spells_cast_last_turn.values().any(|&count| count >= 2)` is always `false` (empty iterator) — Terror of Kruin Pass never transforms back regardless of spells cast.

- **Log message says "Kruin Outlaw transforms into Kruin Outlaw" when back face transforms back to front face** (`mtg-engine/src/cards/isd/kruin_outlaw.rs` lines 103–104)
  - Oracle text says: the back face (Terror of Kruin Pass) transforms into Kruin Outlaw.
  - Code does: `format!("Kruin Outlaw transforms into {}", name)` is used unconditionally. When `is_transformed` is toggled from `true` to `false`, `name` is `"Kruin Outlaw"`, producing the log entry `"Kruin Outlaw transforms into Kruin Outlaw"` instead of `"Terror of Kruin Pass transforms into Kruin Outlaw"`.

### Tricky interactions checked

- **"At the beginning of each upkeep" fires at BOTH players' upkeeps**: PASS — `collect_triggers` processes `GameEvent::StepStarted { step: Step::Upkeep }` for all permanents on the battlefield at every upkeep step, regardless of active player. The trigger fires once per upkeep step for each player's turn.
- **Front-face transform condition ("no spells were cast last turn")**: FAIL — `total_spells_last_turn` is always 0 because `spells_cast_this_turn` is never incremented and `spells_cast_last_turn` is never populated. Front face always transforms on every non-first-turn upkeep.
- **Back-face transform condition ("a player cast two or more spells last turn")**: FAIL — `spells_cast_last_turn` is always empty, so `any(|&count| count >= 2)` is always false. Back face never transforms back.
- **First-turn guard (`!state.is_first_turn`)**: PASS — the front-face condition correctly prevents transformation on the very first turn of the game.
- **Back-face continuous effects only active when transformed**: PASS — `state.rs` `continuous_pt_mods` and `has_keyword` both check `source.is_transformed` before selecting back-face or front-face effects (lines 746–750, 793–797, 832–836, 1055–1059). Menace grant only applies when the card is on its back face.
- **Menace grant filter (Werewolves you control, including self)**: PASS — `CreatureFilter::And([You, HasSubtype("Werewolf")])` in `back_face_data()`. `matches_filter` for `HasSubtype("Werewolf")` checks transformed DFCs via back-face subtypes (line 657–663) and also checks `creature.subtypes` (line 672), so both card-registry entries and token object-level subtypes are covered.
- **"Werewolves you control" includes Terror of Kruin Pass itself**: PASS — scope is `EffectScope::Global` (not `GlobalOther`), so the source itself is included when it passes the filter. Terror's back face has `subtypes: ["Werewolf"]`, so `matches_filter` returns true.
- **Opponent's Werewolves are not granted menace**: PASS — `CreatureFilter::You` restricts the grant to creatures the controller controls.
- **`dynamic_pt` correctly returns 3/3 on back face, None on front**: PASS — `dynamic_pt` checks `obj.is_transformed` and returns `Some((3, 3))` when true, `None` when false. `effective_power/toughness` fall back to `obj.power`/`obj.toughness` (2/2) when `None` is returned.
- **Upkeep trigger correctly dispatched for both faces**: PASS — `trigger_description` finds the front-face `TriggerKind::Upkeep` entry (description `"transform"`) for both faces; `on_upkeep` calls `should_transform` which branches on `is_transformed` to apply the correct condition for each face.
- **Transform check in `on_upkeep` guards on zone**: PASS — `on_upkeep` returns early if `zone != Zone::Battlefield`, so the trigger does nothing if the card is not on the battlefield.
- **`spells_cast_last_turn` per-player vs. total**: PASS (conceptually) — front-face sums across all players (`values().sum()`); back-face checks any single player has `>= 2` (`values().any(|&count| count >= 2)`). Both match the oracle text wording ("no spells" = zero total; "a player cast two or more" = any single player's count >= 2). The logic is correct; the bug is solely that the data is never populated.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Front face transforms when no spells cast last turn: `werewolf_cards.rs:575` (TESTED — but uses default-empty `spells_cast_last_turn`, which is always empty; does not validate engine auto-population)
- Front face does NOT transform when spells cast last turn: `werewolf_cards.rs:61` (TESTED for Reckless Waif via manual `state.spells_cast_last_turn.insert(P0, 1)`; same logic applies to Kruin Outlaw but not directly tested there)
- Front face does NOT transform on first turn (`is_first_turn` guard): `werewolf_cards.rs:48` (TESTED for Reckless Waif; same path)
- Back face transforms when a player cast 2+ spells last turn: `werewolf_cards.rs:641` (TESTED via manual `state.spells_cast_last_turn.insert(P1, 2)`)
- Back face does NOT transform if only 1 spell cast: `werewolf_cards.rs:663` (TESTED via manual `state.spells_cast_last_turn.insert(P0, 1)`)
- Engine actually increments `spells_cast_this_turn` on CastSpell: NOT TESTED (no integration test casts spells via the game loop and then checks the count)
- Engine saves `spells_cast_this_turn` to `spells_cast_last_turn` at turn end: NOT TESTED
- Both players' upkeeps fire the transform trigger: NOT TESTED explicitly
- Terror grants menace to itself: `kruin_outlaw.rs:200` (TESTED)
- Terror grants menace to other Werewolves you control: `kruin_outlaw.rs:200` (TESTED)
- Terror does not affect opponent's Werewolves: `kruin_outlaw.rs:163` (TESTED)
- Terror does not affect non-Werewolves: `kruin_outlaw.rs:127` (TESTED)
- Terror requires 2+ blockers (self): `kruin_outlaw.rs:23` (TESTED)
- Terror allows 2+ blockers: `kruin_outlaw.rs:56` (TESTED)
- Terror grants blocking restriction to other Werewolves: `kruin_outlaw.rs:90` (TESTED)
- Ruling (2011-09-22): "If Kruin Outlaw somehow transforms after blockers have been declared but before combat ends, any Werewolves you control that are blocked by a single creature will remain blocked": NOT TESTED
- Log message accuracy when transforming back to front face: NOT TESTED
