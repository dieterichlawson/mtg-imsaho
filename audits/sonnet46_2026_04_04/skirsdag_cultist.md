## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: `{R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.`
**Type line**: Creature — Human Shaman
**Status**: ISSUE

### Code issues

- **`AnyTarget` does not include planeswalkers as valid targets** — `mtg-engine/src/engine.rs` lines 1343–1358 (activated ability target generation) and lines 1074–1089 (spell target generation)
  - Oracle text says: `This creature deals 2 damage to any target.`
  - Code does: `generate_ability_targets` for `TargetRequirement::AnyTarget` filters for `o.power.is_some()` (creatures only) and then adds players. Planeswalkers — which have no `power` set and are not players — are never included. The `PlayerOrPlaneswalker` arm immediately above (lines 1322–1341) correctly adds planeswalkers, but the `AnyTarget` arm does not. Concretely: `let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter().filter(|o| o.power.is_some())...` — a planeswalker on the battlefield cannot be chosen as a target for the Cultist's ability despite "any target" requiring it.

- **Engine auto-selects which creature to sacrifice instead of presenting a player choice** — `mtg-engine/src/engine.rs` lines 1750–1759
  - Oracle text says: `Sacrifice a creature:` (controller chooses which creature to sacrifice)
  - Code does: `let creature = new_state.objects_in_zone(Zone::Battlefield, player).iter().find(|o| o.power.is_some()).map(|o| o.id);` — the engine auto-picks the first creature found in iteration order (HashMap, unpredictable). When the controller has multiple creatures, they have no agency over which one is sacrificed. The code even contains a TODO comment acknowledging this: `// TODO: Present choice to player when there are multiple options.`

### Tricky interactions checked

- **"Any target" includes planeswalkers**: FAIL — `generate_ability_targets` for `AnyTarget` only generates creatures and players; planeswalkers are absent. The `PlayerOrPlaneswalker` variant correctly handles planeswalkers but `AnyTarget` does not.
- **Controller chooses which creature to sacrifice**: FAIL — engine auto-sacrifices the first creature found (`find` on an unpredictable iteration). No `AwaitingAction` or target-selection mechanism is used for the sacrifice cost.
- **Cultist can sacrifice itself as the cost**: PASS — the legality check (`SacrificeCost::SacrificeCreature`) uses `any(|o| o.power.is_some())` which includes the Cultist itself; the test `skirsdag_cultist_cannot_activate_without_creature` confirms this is allowed.
- **Tap cost correctly enforced**: PASS — `requires_tap: true` is set, and the engine checks `obj_tapped` before generating the action.
- **Once-per-turn restriction absent (correct)**: PASS — `once_per_turn: false` is set; the ability has no such restriction in the oracle text.
- **Sorcery-speed restriction absent (correct)**: PASS — `sorcery_speed_only: false`; the oracle text places no timing restriction.
- **Damage type is NonCombatDamageDealt (not CombatDamageDealt)**: PASS — both the creature-damage and player-damage branches push `GameEvent::NonCombatDamageDealt`.
- **Mana cost {2}{R}{R}**: PASS — `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Red)])`.
- **Subtypes Human and Shaman**: PASS — `subtypes: vec!["Human".into(), "Shaman".into()]`.
- **P/T 2/2**: PASS — `power: Some(2), toughness: Some(2)`.
- **Damage to player correctly reduces life**: PASS — `state.get_player_mut(*player_id).life = old - 2` with `LifeChanged` and `NonCombatDamageDealt` events pushed.
- **Damage to creature marks damage (not instant destroy)**: PASS — `obj.damage_marked += 2`; lethal damage is handled by state-based actions separately.
- **Ability only available while on battlefield**: PASS — `activated_abilities` returns `vec![]` unless `obj.zone == Zone::Battlefield`.

### Test coverage

- **Ability deals 2 damage to a creature**: `tier8_cards.rs:345` — TESTED
- **Ability deals 2 damage to a player**: `tier8_cards.rs:373` — TESTED
- **Cannot activate without a creature to sacrifice**: `tier8_cards.rs:396` — TESTED (confirms Cultist itself counts)
- **`AnyTarget` can target a planeswalker**: NOT TESTED
- **Player chooses which creature to sacrifice when multiple are available**: NOT TESTED
- **Cultist can sacrifice itself and still deal damage**: NOT TESTED (the engine behavior happens to work here, but untested)
- **Tap cost prevents second activation in same turn**: NOT TESTED
