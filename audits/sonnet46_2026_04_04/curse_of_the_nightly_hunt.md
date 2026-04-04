## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
Creatures enchanted player controls attack each combat if able.
**Type line**: Enchantment — Aura Curse
**Status**: ISSUE

### Code issues

- The post-DeclareAttackers forced-attack enforcement loop does not check `state.can_attack()`, causing creatures under "can't attack" effects (e.g., Pacifism) to be illegally added to combat.
  - Oracle text says: `"Creatures enchanted player controls attack each combat if able."`
  - Ruling says: `"If, during the enchanted player's declare attackers step, a creature they control is tapped, is affected by a spell or ability that says it can't attack, or hasn't been under that player's control continuously since the turn began (and doesn't have haste), then it doesn't attack."`
  - Code does: `mtg-engine/src/engine.rs` lines 1825–1847 iterate all battlefield creatures controlled by the active player and filter only for `tapped`, `summoning_sick`, and the `Defender` keyword before adding to the forced list. There is no call to `new_state.can_attack(creature.id, registry)`, which is the function that checks for `ContinuousEffect::PreventAttack` (Pacifism and similar). A creature blocked from attacking by Pacifism would still satisfy all the loop's guards (`!tapped`, `!summoning_sick`, not Defender, ForceAttack effect present from the curse) and would be inserted into `combat.attackers` and tapped as a forced attacker, in violation of the "if able" clause.

  By contrast, `combat::eligible_attackers` (used to build the `must_attack` hint shown to the player in `legal_actions`) correctly calls `state.can_attack()` at line 581, so the Pacifism'd creature would NOT appear in `must_attack`. The player would declare correctly, but the engine would then incorrectly force-add the creature via the post-declaration loop.

### Tricky interactions checked

- **"if able" — tapped creature**: The force loop checks `creature.tapped` at line 1827 and skips, so a tapped creature is correctly exempt. Pass.
- **"if able" — summoning sickness**: The force loop checks `creature.summoning_sick` at line 1827 and skips. Pass.
- **"if able" — Defender keyword**: The force loop checks `has_keyword(...Defender...)` at line 1834 and skips. Pass.
- **"if able" — PreventAttack effect (Pacifism)**: The force loop does NOT call `state.can_attack()`. A Pacifism'd creature controlled by the cursed player is incorrectly forced into combat. Fail.
- **AttachedPlayer filter correctly scopes to enchanted player**: `effect_applies_to` in `state.rs` lines 707–715 reads `source.attached_to_player` and checks `creature.controller == player`. P0's creatures are not cursed; P1's (the attached player) are. Confirmed by the existing test. Pass.
- **P0's own creatures not forced**: The force loop filters `creature.controller != active` (active player = cursed player on their turn), so the caster's creatures are never force-added. Pass.
- **Target validation — hexproof**: `valid_targets_for_req` for `TargetRequirement::PlayerOnly` calls `can_target_player` which blocks targeting a hexproof player (Witchbane Orb). Pass.
- **Curse resolves to battlefield correctly**: `resolve_curse` calls `state.move_object(curse_id, Zone::Battlefield)` and sets `obj.attached_to_player = Some(player_id)`. If no player target is found, falls back to `move_spell_after_resolve`. Pass.
- **Player choice of attack target (ruling 1)**: The ruling says the cursed player still chooses which player/planeswalker each creature attacks. In the `legal_actions` path the player chooses targets via `DeclareAttackers`. For creatures the engine force-adds after declaration, the defending target is hardcoded to `state.opponent(active_player)` (line 1853). In the current 2-player, no-planeswalker environment this is not observable as a bug. Pass for implemented scope.
- **Effect continuously re-evaluates (no snapshot)**: `has_continuous_effect` is called dynamically each time the scope/filter is evaluated; there is no snapshot at ETB. The effect correctly re-evaluates each declare-attackers step. Pass.
- **Curse falls off if moved to graveyard**: `has_continuous_effect` iterates only objects with `zone == Zone::Battlefield`, so the ForceAttack effect stops applying if the curse leaves play. Pass.

### Test coverage

- Basic forced-attack detection (P1's creature has ForceAttack, P0's does not): `mtg-engine/tests/tier7_cards.rs:323` — TESTED
- "if able" — tapped creature not forced: NOT TESTED
- "if able" — summoning-sick creature not forced: NOT TESTED
- "if able" — Defender not forced: NOT TESTED
- "if able" — PreventAttack (Pacifism) creature not forced: NOT TESTED (this is the bug)
- Curse correctly attaches (resolve via cast + targeting): NOT TESTED (test bypasses casting by placing curse directly on battlefield)
- Cursor player's creatures attack; opponent's do not, in full combat flow: NOT TESTED
- Player hexproof blocks targeting: NOT TESTED
