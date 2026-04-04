## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature attacks each combat if able.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
**Type line**: Creature — Vampire
**Status**: ISSUE

### Code issues

- Forced-attack logic in `engine.rs` (~line 1838) does not call `state.can_attack()`, so a Bloodcrazed Neonate enchanted with Pacifism (or any `PreventAttack` effect) is still force-added to combat even though it cannot legally attack.
  - Oracle text says: `"This creature attacks each combat if able."`
  - Code does: iterates battlefield creatures, checks `zone`, `controller`, `power.is_none()`, `tapped`, `summoning_sick`, and `Keyword::Defender`, but **never calls `new_state.can_attack(creature.id, registry)`** before pushing into `forced_ids`. A creature with a `PreventAttack` continuous effect (e.g., Pacifism, Bonds of Faith) passes all those checks and is unconditionally added to `forced_ids`, then inserted into `combat.attackers`. By contrast, the `legal_actions` path correctly computes `eligible` via `combat::eligible_attackers`, which does call `state.can_attack()`, so a Pacifism-enchanted Neonate is absent from `eligible` and `must_attack` there — but `submit_action` overrides that result and adds it to combat anyway.

### Tricky interactions checked

- "if able" — Tapped creature: PASS. Forced-attack loop skips `creature.tapped == true`, so a tapped Neonate is not forced into combat.
- "if able" — Summoning sickness: PASS. Loop skips `creature.summoning_sick == true`.
- "if able" — Defender keyword: PASS. Loop explicitly skips creatures with `Keyword::Defender`.
- "if able" — PreventAttack continuous effect (Pacifism): FAIL. See code issue above; `can_attack()` is never consulted.
- "if able" — Already attacking: PASS. Loop skips creatures already in `combat.attackers`.
- Combat damage trigger fires only when creature damages a player (not a creature): PASS. `collect_triggers` dispatches `CombatDamageToPlayer` only inside the `DamageTarget::Player` branch of `GameEvent::CombatDamageDealt`.
- Trigger collection happens before SBAs kill creatures: PASS. `triggers::process_triggers` is called before the SBA loop in `engine.rs`, so the creature is still on the battlefield when the trigger is collected and resolved.
- Trigger resolves with no effect if Neonate left battlefield: PASS. `on_combat_damage_to_player` guards with `state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before adding the counter.
- Trigger description non-empty (required for dispatch): PASS. `TriggeredAbilityDef` carries description `"put a +1/+1 counter on Bloodcrazed Neonate"`, which is non-empty, satisfying the `!desc.is_empty()` guard in `collect_triggers`.
- EffectScope::OnSelf scopes ForceAttack correctly to only the Neonate itself: PASS. `effect_applies_to` returns `creature_id == source_id` for `OnSelf`, so the effect applies only to the Neonate, not to other permanents.
- Card data (mana cost {1}{R}, 2/1, Vampire subtype, no keywords): PASS.

### Test coverage

- "attacks each combat if able" — ForceAttack effect present: `tier6_cards.rs:273` — TESTED (checks `has_continuous_effect` with `ForceAttack`).
- "attacks each combat if able" — creature is actually forced into combat in a real game step: NOT TESTED (no test submits `DeclareAttackers` and verifies the Neonate ends up attacking).
- "attacks each combat if able" with PreventAttack (Pacifism): NOT TESTED.
- Combat damage trigger puts +1/+1 counter: NOT TESTED (no test fires `CombatDamageDealt` event and verifies a counter is added to the Neonate, unlike the analogous Falkenrath Marauders test nearby in `tier6_cards.rs`).
- Trigger does not fire when damage is dealt to a creature (only to players): NOT TESTED.
- Counter not added if Neonate left battlefield before trigger resolves: NOT TESTED.
