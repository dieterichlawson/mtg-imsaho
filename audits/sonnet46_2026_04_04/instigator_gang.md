## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Attacking creatures you control get +1/+0.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
// Back face (Wildblood Pack): Trample / Attacking creatures you control get +3/+0. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues

- **Engine never increments `spells_cast_this_turn` or updates `spells_cast_last_turn`**: The fields `state.spells_cast_this_turn` and `state.spells_cast_last_turn` are declared in `mtg-engine/src/state.rs` (lines 127, 131) and initialized to empty `HashMap::new()` (lines 230–231), but they are never populated anywhere in the engine. `CastSpell` handling in `mtg-engine/src/engine.rs` (lines 1479–1666) only emits a `GameEvent::SpellCast` event; it never increments `spells_cast_this_turn`. The turn-transition code in `advance_step` (lines 2882–2896) does not swap `spells_cast_this_turn` into `spells_cast_last_turn`. As a result, both maps are permanently empty during any real game.

  Consequence for Instigator Gang (front face, `!is_transformed`): `werewolf_should_transform` at `instigator_gang.rs:13–15` computes `total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` = 0 (always). The guard `total_spells_last_turn == 0 && !state.is_first_turn` is therefore always `true` after the first turn, so Instigator Gang will transform to Wildblood Pack every upkeep — even when spells were cast last turn.
  - Oracle text says: `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`
  - Code does: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum();` — always 0, so condition always fires (engine never populates the map)

  Consequence for Wildblood Pack (back face, `is_transformed`): `werewolf_should_transform` at `instigator_gang.rs:17` checks `state.spells_cast_last_turn.values().any(|&count| count >= 2)` — always `false` (empty map), so Wildblood Pack never transforms back — even when 2+ spells were cast last turn.
  - Oracle text says: `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`
  - Code does: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` — always false (engine never populates the map)

- **Log message names wrong source when transforming back** (`instigator_gang.rs:119–121`): When Wildblood Pack transforms back to Instigator Gang (`was_transformed = true`), the log prints `"Instigator Gang transforms into Instigator Gang"` instead of `"Wildblood Pack transforms into Instigator Gang"`. The destination name is computed correctly (`if was_transformed { "Instigator Gang" }`), but the source string is hardcoded as `"Instigator Gang"` regardless of direction.
  - Oracle text says: (transformation is bidirectional between two named faces)
  - Code does: `format!("Instigator Gang transforms into {}", name)` at line 121 — "Instigator Gang" is always the stated source, even when the source face is Wildblood Pack

### Tricky interactions checked

- **Front-to-back transform condition (no spells last turn)**: FAIL — engine never tracks `spells_cast_this_turn` nor copies it to `spells_cast_last_turn`, so the transform fires unconditionally after turn 1 regardless of whether spells were actually cast.
- **Back-to-front transform condition (2+ spells by any player)**: FAIL — same root cause; `spells_cast_last_turn` is always empty so Wildblood Pack never reverts.
- **First-turn guard prevents transformation on turn 1**: PASS — `!state.is_first_turn` at `instigator_gang.rs:15` correctly blocks transform on the very first upkeep.
- **Each upkeep (both players)**: PASS — `TriggerKind::Upkeep` is dispatched from `GameEvent::StepStarted { step: Step::Upkeep }` in `triggers.rs:598–643`, scanning all battlefield permanents regardless of active player. Upkeep trigger in front face `triggered_abilities` (line 41–44) is found by `trigger_description`; for the transformed face, the front face check returns it first so the trigger still fires.
- **Wildblood Pack upkeep trigger dispatch (back face missing Upkeep in triggered_abilities)**: PASS — `trigger_description` in `triggers.rs:312–327` checks front face first; the front face has `TriggerKind::Upkeep` so it returns a non-empty description even when `is_transformed = true`. The trigger correctly calls `on_upkeep` for both faces.
- **Attacking creatures you control get +1/+0 (+3/+0 when transformed)**: PASS — `on_any_creature_attacks` reads `is_transformed` at resolution time and applies the appropriate bonus (1 or 3). Attacker-self is not excluded (by design, comment at `triggers.rs:724`), so Instigator Gang/Wildblood Pack buffs itself when attacking.
- **Buff only applies to creatures you control**: PASS — `on_any_creature_attacks` at `instigator_gang.rs:95–97` compares `attacker_controller != controller` and returns early for enemy creatures.
- **Until-end-of-turn cleanup of attack buffs**: PASS — `until_end_of_turn_effects` is a Vec pushed to at lines 99–105 and read in `effective_power`/`effective_toughness`; the engine's end-step cleanup removes these entries.
- **AttackWatch trigger correctly uses `is_transformed` to determine bonus at resolution time**: PASS — `on_any_creature_attacks` re-reads `is_transformed` from the live object, not from a snapshot, so bonus is correct regardless of when the trigger resolves relative to state changes.
- **Back face Trample keyword**: PASS — `back_face_data` declares `keywords: vec![Keyword::Trample]` (line 64). `apply_transform` in `helpers.rs:257–263` copies `back.keywords` to `obj.keywords` when transforming, so `has_keyword(Keyword::Trample)` returns true for Wildblood Pack.
- **Back face P/T (5/5)**: PASS — `dynamic_pt` returns `Some((5, 5))` when `is_transformed = true` (lines 81–87), which `effective_power`/`effective_toughness` uses as the base, overriding `obj.power`. Tests confirm 5/5.
- **Transform log message (front→back)**: PASS — when `was_transformed = false`, logs "Instigator Gang transforms into Wildblood Pack". Correct.
- **Transform log message (back→front)**: FAIL — when `was_transformed = true`, logs "Instigator Gang transforms into Instigator Gang" instead of "Wildblood Pack transforms into Instigator Gang".

### Test coverage

- Front-to-back transform (no spells last turn): `werewolf_cards.rs:453` (`instigator_gang_transforms_and_gains_trample`) — tests the transform but bypasses the engine spell-tracking bug by not setting `spells_cast_last_turn` (empty = no spells = transforms). Does NOT test that spells actually prevent the transform in a real game.
- Front face stays human when spells were cast: NOT TESTED for Instigator Gang specifically. `reckless_waif_stays_human_when_spells_cast` (`werewolf_cards.rs:61`) covers this for Reckless Waif by manually setting `spells_cast_last_turn`.
- Back-to-front transform (2+ spells): NOT TESTED for Instigator Gang.
- Buff self when attacking (front): `werewolf_cards.rs:469` (`instigator_gang_buffs_itself_when_attacking`) — TESTED.
- Buff other ally when attacking (front): `werewolf_cards.rs:489` (`instigator_gang_buffs_other_attackers_you_control`) — TESTED.
- Does not buff opponent attackers: `werewolf_cards.rs:510` (`instigator_gang_does_not_buff_opponent_attackers`) — TESTED.
- Wildblood Pack +3/+0 buff when attacking: `werewolf_cards.rs:528` (`wildblood_pack_buffs_itself_plus_3`) — TESTED.
- Wildblood Pack has Trample: `werewolf_cards.rs:464` — TESTED.
- Engine spell count tracking (real game, not manual state injection): NOT TESTED — no integration test casts a spell and then checks whether the werewolf condition is properly evaluated.
- Log message correctness (back→front): NOT TESTED.
