## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +2/+2 and attacks each combat if able.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues

- Missing "Enchant creature\n" prefix in `oracle_text` field — `mtg-engine/src/cards/isd/furor_of_the_bitten.rs:22`
  - Oracle text says: `"Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able."`
  - Code does: `oracle_text: "Enchanted creature gets +2/+2 and attacks each combat if able.".into()`
  - All other auras in the same set that have been verified (dead_weight, curiosity, sensory_deprivation, wreath_of_geists, claustrophobia) include the "Enchant creature\n" prefix. Furor omits it. This is a display mismatch with oracle text.

- Forced-attack enforcement ignores Haste overriding summoning sickness — `mtg-engine/src/engine.rs:1827`
  - Oracle text says: `"attacks each combat if able"` (ruling: "If the enchanted creature can't attack for any reason (such as being tapped or having come under that player's control that turn), then it doesn't attack.")
  - Code does: `|| creature.summoning_sick {  continue; }` — unconditionally skips summoning-sick creatures without checking for Haste.
  - By contrast, `eligible_attackers` in `combat.rs:577` correctly uses `(!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry))`. A creature enchanted by Furor that has Haste AND summoning sickness CAN legally attack ("is able"), so Furor should force it to attack. The `must_attack` vec in `legal_actions` (which uses `eligible_attackers`) correctly includes such a creature, but the post-declaration enforcement code at engine.rs:1827 would skip it — the engine says it must attack but then doesn't enforce it.

- Forced-attack enforcement skips `can_attack()` check — `mtg-engine/src/engine.rs:1838–1846`
  - Oracle text says: `"attacks each combat if able"`
  - Code does: checks only for the `ForceAttack` continuous effect, Defender keyword, tapped, and summoning sick. It does NOT call `state.can_attack()`, which checks for `PreventAttack` continuous effects (e.g., Pacifism) and instance_oracle_text locks (e.g., Bonds of Faith on a non-Human).
  - If a creature has both `ForceAttack` (Furor) and a `PreventAttack` effect (Pacifism on the same creature, or Bonds of Faith on a non-Human), the `eligible_attackers` function correctly excludes it (the creature is "not able" to attack), but the `forced_ids` code would still add it to combat — forcing an attack that the creature is not able to make.

### Tricky interactions checked

- "+2/+2 bonus continuously re-evaluated via EffectScope::Attached**: pass — `effect_applies_to` checks `source.attached_to == creature_id` at runtime, so if the aura falls off the effect stops.
- "attacks each combat if able" — tapped creature exempt: pass — `forced_ids` checks `creature.tapped` at engine.rs:1827.
- "attacks each combat if able" — summoning-sick creature (came under control this turn) exempt: pass for non-Haste case — `forced_ids` checks `creature.summoning_sick` at engine.rs:1827.
- "attacks each combat if able" — summoning-sick creature WITH Haste must still attack: FAIL — `forced_ids` at engine.rs:1827 skips on `summoning_sick` without checking Haste; `eligible_attackers` correctly handles this but `forced_ids` does not.
- "attacks each combat if able" — creature with PreventAttack effect exempt: FAIL — `forced_ids` does not call `can_attack()`, so a creature with Furor + Pacifism would be incorrectly force-attacked.
- Aura falls off when enchanted creature leaves battlefield: pass — `resolve_aura` uses `move_spell_after_resolve` when target is gone; engine handles aura detachment via state-based actions.
- Target must be a creature: pass — `target_requirement` returns `TargetRequirement::Creature`.
- Vigilance interaction with forced attack: pass — `forced_ids` code checks `has_keyword(Vigilance)` before tapping (engine.rs:1864).
- Defender interaction with forced attack: pass — `forced_ids` code checks `has_keyword(Defender)` and skips (engine.rs:1834).
- Multiple auras stacking (two Furors on same creature): pass — `has_continuous_effect` checks all battlefield sources; creature correctly ends up in `forced_ids` only once because `contains_key` guard prevents double-insertion (engine.rs:1856).
- oracle_text field includes "Enchant creature" prefix: FAIL — prefix missing (furor_of_the_bitten.rs:22).

### Test coverage

- +2/+2 bonus applied on enchanting: `innistrad_cards.rs:358` (furor_of_the_bitten_gives_plus_two), `bug_fixes.rs:497` (furor_of_the_bitten_gives_plus_two_and_forces_attack)
- ForceAttack continuous effect present after enchanting: `bug_fixes.rs:512`, `card_mechanics.rs:241` (furor_forces_attack)
- Creature auto-added to combat when player declares zero attackers: `card_mechanics.rs:241` (furor_forces_attack)
- Tapped creature exempt from forced attack: NOT TESTED
- Summoning-sick creature exempt from forced attack: NOT TESTED
- Summoning-sick creature WITH Haste forced to attack: NOT TESTED
- PreventAttack + ForceAttack interaction ("if able" with Pacifism): NOT TESTED
- Bonds-of-Faith lock + ForceAttack interaction: NOT TESTED
- Aura falls off when enchanted creature leaves battlefield: NOT TESTED
- oracle_text field includes "Enchant creature" prefix: NOT TESTED
