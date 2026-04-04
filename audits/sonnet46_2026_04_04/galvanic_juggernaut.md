## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature attacks each combat if able.
This creature doesn't untap during your untap step.
Whenever another creature dies, untap this creature.
**Type line**: Artifact Creature — Juggernaut
**Status**: ISSUE

### Code issues

- Forced attack logic ignores `can_attack()`, violating the "if able" clause
  - File: `mtg-engine/src/engine.rs` lines 1822–1847 (DeclareAttackers forced-attacker loop)
  - Oracle text says: `"This creature attacks each combat if able."`
  - Code does: The loop checks `creature.tapped`, `creature.summoning_sick`, and `has_keyword(Defender)` as "unable" conditions, but does **not** call `state.can_attack(creature.id, registry)`. `can_attack` checks for `ContinuousEffect::PreventAttack` (e.g., Pacifism) and `instance_oracle_text` containing `"can't attack or block"` (e.g., Bonds of Faith on a non-Human, Claustrophobia). As a result, if Galvanic Juggernaut is enchanted with Pacifism (which registers `ContinuousEffect::PreventAttack { scope: EffectScope::Attached }`), it is still auto-added to the forced-attacker list and made to attack illegally. Note that `combat::eligible_attackers` (used by `legal_actions`) **does** call `can_attack` and correctly excludes the Pacifism'd Juggernaut from `must_attack`, so there is an inconsistency: the UI tells the player the Juggernaut need not attack, but the engine forces it to attack anyway when `submit_action(DeclareAttackers)` is processed.

### Tricky interactions checked

- **"another" creature dies — self-exclusion**: The dispatch in `triggers.rs` (line 419) filters `o.id != dead_id` and also requires `o.zone == Zone::Battlefield`. Because the dying creature is moved to the graveyard before `collect_triggers` runs, the Juggernaut cannot receive a DeathWatch trigger when it itself dies. PASS
- **Simultaneous deaths (board wipe, Juggernaut survives)**: Multiple `CreatureDied` events are emitted sequentially; for each one the watcher list is re-scanned from the current battlefield. As long as the Juggernaut is still on the battlefield it is included in watchers for each death event, generating one `DeathWatch` trigger per death. The `on_any_creature_dies` handler only untaps when `obj.tapped == true`, so repeated triggers after the first are no-ops. PASS
- **"if able" with tapped / summoning sickness / Defender**: The forced-attack loop (engine.rs 1826–1836) correctly skips the Juggernaut when it is tapped, summoning-sick, or has Defender. `eligible_attackers` also excludes it from the `must_attack` list in those cases. PASS
- **"if able" with PreventAttack (Pacifism)**: As described in Code Issues above, `can_attack()` is not called in the forced-attack loop. A Pacifism'd Juggernaut is incorrectly forced to attack. FAIL
- **PreventUntap during untap step**: `engine.rs` `perform_turn_based_actions` (line 2916) correctly reads `has_continuous_effect(PreventUntap)` and excludes the Juggernaut from the untap batch. PASS
- **Trigger fires while Juggernaut is untapped**: The `on_any_creature_dies` handler checks `obj.tapped` before untapping. If the Juggernaut is already untapped the handler is a no-op. This matches MTG rules (untapping an untapped permanent does nothing). PASS
- **Watcher resolves if Juggernaut leaves battlefield before trigger resolves**: `resolve_next_trigger` (triggers.rs line 908) checks `o.zone == Zone::Battlefield` before calling `on_any_creature_dies`. If the Juggernaut leaves between trigger collection and resolution the effect is correctly skipped. PASS
- **`ForceAttack` scope OnSelf only applies to Juggernaut**: `effect_applies_to` returns true for `EffectScope::OnSelf` only when `creature_id == source_id` (state.rs line 699), so the ForceAttack effect cannot accidentally apply to other creatures. PASS
- **DeathWatch trigger collection for all permanents**: The death-watch path in `triggers.rs` creates `DeathWatch` triggers for every registered permanent on the battlefield (no `desc.is_empty()` guard, unlike `EnterWatch`). Default `on_any_creature_dies` is a no-op, so spurious triggers fire harmlessly. This is a performance issue but not a correctness issue for the Juggernaut. PASS

### Test coverage

- Untap trigger fires when another creature dies: `mtg-engine/tests/tier15_cards.rs:143` — TESTED
- Untap trigger does not fire when Juggernaut itself dies: NOT TESTED
- ForceAttack causes Juggernaut to be declared as a forced attacker: NOT TESTED
- "if able" — Juggernaut not forced to attack when tapped: NOT TESTED
- "if able" — Juggernaut not forced to attack under Pacifism (the identified bug): NOT TESTED
- PreventUntap skips Juggernaut during untap step: NOT TESTED
- Multiple simultaneous deaths each generate a separate trigger: NOT TESTED
