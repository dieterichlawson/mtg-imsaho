## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- **fight() emits CombatDamageDealt instead of NonCombatDamageDealt, applying combat-specific effects to fight damage** (`mtg-engine/src/combat.rs:467`, `mtg-engine/src/combat.rs:429–436`, `mtg-engine/src/combat.rs:452–454`, `mtg-engine/src/combat.rs:435–437`)
  - Oracle text says: `(Each deals damage equal to its power to the other.)` — this is reminder text for the fight keyword action, which is non-combat damage. The engine itself distinguishes the two in `events.rs:27–30`: `CombatDamageDealt` vs `/// Non-combat damage dealt (e.g., triggered abilities, spells). NonCombatDamageDealt`.
  - Code does: `deal_damage_to_creature` in `fight()` emits `GameEvent::CombatDamageDealt { source, target: DamageTarget::Object(target), amount, }` (line 467–471). Because `fight()` calls the same `deal_damage_to_creature` function used for actual combat, three combat-only modifiers are incorrectly applied to fight damage:
    1. `has_damage_prevention` (`PreventCombatDamage`) at line 430: `if has_damage_prevention(state, source, registry) || has_damage_prevention(state, target, registry) { return; }` — Ghostly Possession's `ContinuousEffect::PreventCombatDamage` effect incorrectly prevents fight damage.
    2. `is_non_wolf_damage_prevented` at line 435: `if is_non_wolf_damage_prevented(state, source, registry) { return; }` — Moonmist's combat-damage-only prevention incorrectly blocks fight damage from non-Wolf/Werewolf creatures.
    3. `combat_damage_multiplier` at lines 452–454: `amount *= combat_damage_multiplier(state, source, registry); amount *= combat_damage_multiplier(state, target, registry);` — Inquisitor's Flail's `DoubleCombatDamage` effect incorrectly doubles fight damage.
  - Additionally, because `CombatDamageDealt` (not `NonCombatDamageDealt`) is emitted, the trigger system (`triggers.rs:459–487`) fires `DealsCombatDamageToCreature` triggers during fight. Concretely: if Creepy Doll (which has `TriggerKind::DealsCombatDamageToCreature`) is made to fight via Prey Upon, its "flip a coin; if you win, destroy that creature" ability incorrectly fires, violating Creepy Doll's oracle text ("Whenever Creepy Doll deals **combat** damage to a creature").

- **One illegal target does not prevent fight, violating the Scryfall ruling** (`mtg-engine/src/stack.rs:79–86`, `mtg-engine/src/cards/isd/prey_upon.rs:35–52`, `mtg-engine/src/combat.rs:158–168`)
  - Oracle text says (ruling): `If either target is an illegal target as Prey Upon resolves, no creature will deal or be dealt damage.`
  - Code does: `stack.rs:80`: `let any_legal = targets.iter().any(|t| is_target_legal(state, t, &target_req)); if !any_legal { state.log(..., format!("{} fizzled (all targets illegal)", name)); ... return; }` — the spell only fizzles when **all** targets are illegal. If exactly one target becomes illegal (e.g., the opponent's creature dies in response), `on_resolve` is called with both target IDs. `prey_upon.rs:48` then calls `crate::combat::fight(state, my_creature, their_creature, registry)` with no battlefield check. Inside `fight()` (lines 158–168), `state.effective_power(b, registry)` does not filter by zone: `let obj = self.get_object(id)?;` returns the graveyard object (objects are kept in `state.objects` with their `power` field intact after `move_object`). So `power_b > 0` is true, `deal_damage_to_creature(state, b, a, power_b, registry)` is called, and `state.get_object_mut(a)` returns the living creature, marking `power_b` damage on it. The living creature incorrectly receives damage from a dead creature, directly violating the ruling.

### Tricky interactions checked

- **Both targets legal, normal fight**: PASS — `on_resolve` calls `fight`, each creature marks damage equal to the other's effective power.
- **Both targets illegal (all targets fizzle)**: PASS — `stack.rs` fizzles and `on_resolve` is never called.
- **One target illegal (partial illegality)**: FAIL — engine only checks `any_legal`, calls `on_resolve` with the dead target, `fight()` reads graveyard creature's power and marks damage on the surviving creature. Violates the ruling.
- **Fight damage event type**: FAIL — `deal_damage_to_creature` emits `CombatDamageDealt`; should emit `NonCombatDamageDealt` for damage from the fight keyword action on a sorcery spell.
- **Ghostly Possession + fight**: FAIL — `has_damage_prevention` in `deal_damage_to_creature` checks `PreventCombatDamage`, incorrectly preventing fight damage to/from a Ghostly Possession-enchanted creature.
- **Inquisitor's Flail + fight**: FAIL — `combat_damage_multiplier` applied inside `deal_damage_to_creature` doubles fight damage, incorrect since fight is not combat.
- **Moonmist + fight**: FAIL — `is_non_wolf_damage_prevented` applies inside `deal_damage_to_creature`, preventing non-Wolf/Werewolf fight damage, which is incorrect.
- **Creepy Doll fights via Prey Upon**: FAIL — `CombatDamageDealt` event fires `DealsCombatDamageToCreature` trigger in `triggers.rs:468`, causing Creepy Doll's coin-flip ability to incorrectly fire during fight.
- **`move_spell_after_resolve` called**: PASS — `prey_upon.rs:51` calls `state.move_spell_after_resolve(object_id)`, correctly placing the spell in the graveyard after resolution.
- **Targeting — YouControl / YouDontControl**: PASS — `target_requirement` returns `TwoTargets(CreatureWithFilter(YouControl), CreatureWithFilter(YouDontControl))`, matching the oracle text.
- **Mana cost, card type, oracle text field**: PASS — `{G}`, Sorcery, oracle text string all match the Scryfall data.
- **Power 0 or negative creatures**: PASS — `fight()` uses `.unwrap_or(0).max(0)` and only calls `deal_damage_to_creature` if `power > 0`, so 0-power creatures deal no damage.
- **Target ordering (mine first vs theirs first)**: PASS — `on_resolve` checks `a_mine` and swaps ordering as needed.

### Test coverage

- Normal fight (3/3 fights 2/2, correct damage marked): `mtg-engine/tests/tier2_spells.rs:336` (`prey_upon_fight`)
- One target becomes illegal before resolution, neither creature deals damage: NOT TESTED
- Both targets illegal (full fizzle): NOT TESTED
- Fight damage does not trigger Creepy Doll's `DealsCombatDamageToCreature` ability: NOT TESTED
- Fight damage not doubled by Inquisitor's Flail: NOT TESTED
- Fight damage not prevented by Ghostly Possession: NOT TESTED
- Fight damage not prevented by Moonmist: NOT TESTED
