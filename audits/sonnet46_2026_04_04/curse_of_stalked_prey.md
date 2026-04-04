## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Mandatory vs "may"**: Oracle says "put a +1/+1 counter on that creature" (no "may"). Code adds counter unconditionally in `on_any_combat_damage_to_player` — no choice presented. Correct: pass.
- **Any creature, any controller**: Ruling states the ability triggers for creatures controlled by any player (including the enchanted player or another opponent). Code checks only `cursed_player != Some(damaged_player)` with no restriction on the attacking creature's controller. Correct: pass.
- **Trigger dispatch includes non-creature permanents**: In `triggers.rs` line 518-521, the watcher scan for `AnyCombatDamageToPlayer` filters by `zone == Battlefield` only (no `power.is_some()` filter), so Curse of Stalked Prey (an enchantment with no P/T) is correctly included in the watcher list. Correct: pass.
- **Trigger description non-empty gating**: `trigger_description` returns "put a +1/+1 counter on that creature" for `TriggerKind::AnyCombatDamageToPlayer`, which is non-empty, so the trigger IS collected. Correct: pass.
- **Curse checks own zone at resolution**: `on_any_combat_damage_to_player` at line 50-53 verifies the curse object is in `Zone::Battlefield` before reading `attached_to_player`. Correct: pass.
- **Source creature leaves battlefield before trigger resolves**: Code at line 58 checks `state.get_object(source_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before adding the counter. If the creature is gone, the counter is not added. This is correct MTG rules behavior for a non-targeted ability that refers to a specific object. Correct: pass.
- **Curse not attached to any player (attached_to_player = None)**: Condition `None != Some(damaged_player)` evaluates to `true`, causing early return. Counter not applied. Correct: pass.
- **resolve_curse sets attached_to_player**: `helpers.rs` line 38 sets `obj.attached_to_player = Some(*player_id)` after moving to battlefield. The `on_any_combat_damage_to_player` reads this field. Correct: pass.
- **TargetRequirement::PlayerOnly**: Curse targets a player at cast time (correct for "Enchant player"). Correct: pass.
- **CombatDamageDealt event path**: `combat.rs` line 513 emits `GameEvent::CombatDamageDealt { target: DamageTarget::Player(player), .. }` when a creature deals damage to a player. `collect_triggers` in `triggers.rs` line 489 handles this event and dispatches `CombatDamageWatch` triggers to all `AnyCombatDamageToPlayer` watchers. Correct: pass.
- **No keywords declared**: Oracle "Keywords: Enchant" from Scryfall is an ability word, not an engine keyword. `keywords: vec![]` in card data is correct. Correct: pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Counter added to creature that dealt combat damage to enchanted player: `tier15_cards.rs:23` — TESTED
- Trigger fires for creatures controlled by another opponent: NOT TESTED
- Trigger fires for creatures controlled by the enchanted player: NOT TESTED
- Counter not added when source creature leaves battlefield before resolution: NOT TESTED
- Curse no-ops when not attached to any player: NOT TESTED
- Full engine path (CombatDamageDealt event → collect_triggers → resolve_next_trigger → counter): NOT TESTED (test calls `on_any_combat_damage_to_player` directly, bypassing trigger collection)
