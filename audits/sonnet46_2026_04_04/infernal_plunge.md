## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Add {R}{R}{R}.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Sacrifice at cast time, not resolution**: PASS. `submit_action` in `engine.rs` (lines 1541–1546) calls `crate::destruction::sacrifice` before the spell enters the stack. The test `sacrifice_at_cast_time` in `infernal_plunge.rs:62` confirms the creature is in Zone::Graveyard while the spell is still on the stack.
- **Player choice of which creature to sacrifice**: PASS. `legal_actions` in `engine.rs` (lines 576–590) expands each untargeted cast action into one `CastSpell { sacrifice: Some(sac_id) }` per eligible creature, giving the player a distinct action for each choice. Tested by `one_action_per_sacrifice_target` in `infernal_plunge.rs:122`.
- **Cannot cast without a creature to sacrifice**: PASS. `engine.rs` line 536: `if creatures.is_empty() { continue; }` skips Infernal Plunge from legal actions when the controller has no creatures. Tested by `cannot_cast_without_creature` in `infernal_plunge.rs:20`.
- **Sacrifice filters to controller's own creatures only**: PASS. `objects_in_zone(Zone::Battlefield, player)` (`state.rs` line 604) filters by `controller == player` for the Battlefield zone, so only the casting player's creatures appear as sacrifice options.
- **Cannot sacrifice additional creatures (exactly one)**: PASS. Each generated `CastSpell` action carries exactly one `sacrifice: Some(sac_id)`. The engine processes only that one at cost-payment time.
- **Mana added is {R}{R}{R} (three red, not colorless or other color)**: PASS. `on_resolve` calls `state.get_player_mut(controller).mana_pool.add(ManaType::Red, 3)` exactly.
- **Spell moves to graveyard (not exile) after resolution**: PASS. `move_spell_after_resolve` in `state.rs` (lines 1132–1141) sends non-flashback spells to `Zone::Graveyard`. Infernal Plunge has no flashback cost, so `cast_with_flashback` is false.
- **Sacrifice bypasses indestructible and regeneration**: PASS. `destruction::sacrifice` in `destruction.rs` (line 63) calls `destroy` directly, skipping the indestructible and regeneration checks in `try_destroy`.
- **Backward compatibility path (sacrifice: None submitted directly)**: Non-issue in gameplay. The backward compat path at `engine.rs` lines 1548–1566 auto-selects the first creature when `sacrifice: None` is submitted for a card with `AdditionalCost::SacrificeCreature`. This path is only reachable by directly constructing a `CastSpell { sacrifice: None }` action (e.g., via `cast_and_resolve` helper in tests), not through `legal_actions` which always produces `sacrifice: Some(sac_id)`. The `tier8_cards.rs:196` test exercises this path and the behavior is still correct (creature sacrificed, RRR added).

### Test coverage
- Cannot cast without a creature: `infernal_plunge.rs:20` ✓
- Can cast when controlling a creature: `infernal_plunge.rs:41` ✓
- Sacrifice happens at cast time (creature gone before resolution): `infernal_plunge.rs:62` ✓
- Adds {R}{R}{R} on resolution: `infernal_plunge.rs:92` and `tier8_cards.rs:196` ✓
- One CastSpell action per eligible sacrifice target: `infernal_plunge.rs:122` ✓
- Sacrifice bypasses indestructible/regeneration: NOT TESTED
- Spell ends up in graveyard after resolution: NOT TESTED (implicitly covered by `tier8_cards.rs:196` since resolve is called but zone not asserted)
- Player cannot sacrifice opponent's creatures: NOT TESTED
