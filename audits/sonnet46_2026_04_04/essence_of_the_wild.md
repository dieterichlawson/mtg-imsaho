## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creatures you control enter as a copy of this creature.
**Type line**: Creature — Avatar
**Status**: ISSUE

### Code issues

- **ETB abilities of creatures entering as EotW copies still fire** — `mtg-engine/src/triggers.rs:344-392` and `mtg-engine/src/state.rs:524-575`
  - Oracle text says: `"Because creatures you control enter as copies of Essence of the Wild, any 'enters' triggered abilities printed on such creatures won't trigger."` (official ruling 2011-09-22)
  - Code does: `apply_entering_copy_replacement` (state.rs:524) updates the entering creature's name, power, toughness, colors, card_types, subtypes, keywords, and instance_oracle_text — but does NOT update `card_id`. In `collect_triggers` (triggers.rs:345), `card_id` is read from `o.card_id` (the original card), and `PendingTrigger::EnteredBattlefield { card_id }` is pushed with that original card_id. When resolved (triggers.rs:893-899), `registry.get(card_id).on_enter_battlefield(state, object_id, registry)` calls the **original card's** ETB behavior, not EotW's (which is a no-op). For example, Hollowhenge Scavenger (`mtg-engine/src/cards/isd/hollowhenge_scavenger.rs:38`) has `triggered_abilities: [TriggerKind::EntersBattlefield]` and overrides `on_enter_battlefield` to gain 5 life if morbid. If Hollowhenge Scavenger enters while EotW is on the battlefield, it correctly becomes a 6/6 Avatar (EotW copy), but the Morbid ETB still fires incorrectly. Same for Morkrut Banshee (`morkrut_banshee.rs:38`) and Fiend Hunter (`fiend_hunter.rs:43`) and all other cards with `on_enter_battlefield` overrides.

- **EotW entering via non-`on_resolve` path does not apply replacement effect** — `mtg-engine/src/cards/isd/essence_of_the_wild.rs:40-53`
  - Oracle text says: `"Creatures you control enter as a copy of this creature."` — a continuous replacement effect that applies whenever EotW is on the battlefield, regardless of how it arrived.
  - Code does: The `entering_copy_source` flag (which is what `apply_entering_copy_replacement` in state.rs:540 checks) is only set to `true` in `on_resolve` (essence_of_the_wild.rs:46: `obj.entering_copy_source = true`). If EotW enters the battlefield via any other path — for instance, reanimated by Unburial Rites after having been countered (where `on_resolve` never ran and the flag was never set) — the flag remains `false`. Subsequent creatures entering under the controller's control will not be replaced, violating the oracle text. Note: if EotW previously resolved normally, its flag persists through the graveyard (not cleared on LTB), so death-then-reanimate works correctly. The bug only triggers when EotW was countered (or otherwise moved to the graveyard without resolving) before reanimation.

### Tricky interactions checked

- **EotW entering while another EotW is already on the battlefield**: The entering EotW gets `entering_copy_source = true` via `apply_entering_copy_replacement` (from the existing EotW), then `on_resolve` sets it to `true` again (redundant but harmless). Subsequent creatures enter as copies of either EotW — both have the flag. PASS.
- **Opponent's creatures not affected**: `apply_entering_copy_replacement` checks `o.controller == controller` (state.rs:539), so only the EotW controller's creatures are affected. PASS.
- **EotW itself not self-replacing**: `entering_copy_source` is set to `true` AFTER `move_object` returns (essence_of_the_wild.rs:46). During `apply_entering_copy_replacement`, the flag is still `false` on the entering EotW, so it can't be found as a copy source for itself. The `o.id != entering_id` guard (state.rs:541) also prevents self-copy. PASS.
- **Tokens entering while EotW is on the battlefield**: `create_token_with_subtypes` calls `apply_entering_copy_replacement` (state.rs:402). Tokens have `power: Some(...)` set, so `is_creature` check passes (state.rs:527). Tokens correctly enter as EotW copies. PASS.
- **Token copies of EotW (Cackling Counterpart targeting EotW) acting as copy sources**: `create_token_copy` creates a token with EotW's `card_id`. `apply_entering_copy_replacement` runs and copies `entering_copy_source = true` from the existing EotW to the token. The token also acts as a copy source. PASS.
- **Non-creature permanents not affected**: `apply_entering_copy_replacement` checks `o.power.is_some()` (state.rs:527). Non-creature permanents (lands, artifacts, enchantments) have `power: None` from their card data, so they are correctly skipped. PASS.
- **Tapped/counter state of EotW not copied**: `apply_entering_copy_replacement` reads `source.power.unwrap_or(0)` (base printed power, not effective power). Counters are stored separately in `counters` map and are not included. Tapped state and attached auras/equipment are not read. PASS per ruling 2011-09-22.
- **ETB abilities of entering creatures not suppressed** (primary issue): When a creature with `triggered_abilities: [TriggerKind::EntersBattlefield]` enters as an EotW copy, the trigger is collected using the original `card_id` and the original card's `on_enter_battlefield` is called. FAIL — see Issue 1 above.
- **EotW flag persists through death correctly**: When EotW dies and goes to the graveyard, `move_object` does not clear `entering_copy_source`. If it was `true` (had resolved normally), it stays `true` in the graveyard. On reanimation, the flag is correctly still `true`. PASS for this specific path.
- **Replacement effect not applied when EotW enters via reanimation after being countered**: FAIL — see Issue 2 above.
- **`instance_continuous_effects` cleared on entering creatures**: Set to `Some(vec![])` for the entering creature (state.rs:568), clearing any registry-based continuous effects the original creature would have had. EotW has no continuous effects, so this is correct. PASS.

### Test coverage

- Basic replacement (vanilla creature enters as EotW copy): `tier15_cards.rs:2507` — TESTED (`essence_overrides_entering_creatures`)
- Opponent's creatures not affected: `tier15_cards.rs:2532` — TESTED (`essence_does_not_override_opponent_creatures`)
- ETB abilities of entering creatures should not fire: NOT TESTED
- Tokens entering as EotW copies: NOT TESTED
- Multiple EotW on the battlefield: NOT TESTED
- EotW entering via reanimation (non-resolve path): NOT TESTED
- EotW countered then reanimated (entering_copy_source never set): NOT TESTED
- Tapped/counter state of EotW not copied to entering creatures: NOT TESTED
- Ruling 2011-09-22 (external abilities like Urabrask still apply): NOT TESTED
- Ruling 2011-09-22 (Clone/copy effects result in EotW): NOT TESTED
