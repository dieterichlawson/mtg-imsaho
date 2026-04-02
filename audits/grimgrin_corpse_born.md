# Audit: Grimgrin, Corpse-Born

## Oracle Reference (Scryfall)
- Cost: {3}{U}{B}
- Type: Legendary Creature -- Zombie Warrior
- P/T: 5/5
- Oracle: "Grimgrin, Corpse-Born enters tapped and doesn't untap during your untap step.
  Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
  Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin."

## Implementation: grimgrin_corpse_born.rs

## Issues Found

1. **ISSUE: Sacrifice ability auto-selects sacrifice target** - The sacrifice ability should let the player choose which creature to sacrifice. The implementation auto-picks "the first available that isn't Grimgrin" (line 83-86). This removes strategic choice.

2. **ISSUE: Sacrifice ability has no sacrifice_cost** - The activated ability definition (line 64-73) sets sacrifice_cost to SacrificeCost::None. The actual sacrifice happens in on_activate_ability. This means the engine might not properly enforce the "sacrifice another creature" cost -- if no other creatures exist, the ability shouldn't be activatable, but it still appears in the list.

3. **ISSUE: Attack trigger auto-targets** - The attack trigger should let the player choose which creature to destroy. The implementation auto-selects "the first creature the defending player controls" (line 110-113).

4. **ISSUE: Oracle text order** - Oracle says "destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin." The implementation correctly destroys first (line 117), then adds the counter (line 123). The counter should only be added if the destroy happened (targeting is mandatory). Currently the counter is always added even if no target exists.

Otherwise correct: cost, types (Legendary Zombie Warrior), P/T (5/5), enters tapped, doesn't untap, sacrifice-to-untap ability, attack trigger.

## Verdict: ISSUES FOUND (4 issues)

## Audit — 2026-04-01 15:30

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Mana cost**: {3}{U}{B}
**P/T**: 5/5
**Rulings**:
- [2013-07-01] If Grimgrin's last ability resolves, but the targeted creature isn't destroyed (perhaps because it regenerated or has indestructible), you'll still put a +1/+1 on Grimgrin.
- [2011-09-22] If the targeted creature is an illegal target by the time Grimgrin's last ability resolves, the entire ability doesn't resolve and none of its effects will occur. You won't put a +1/+1 counter on Grimgrin.
- [2011-09-22] If the defending player controls no creatures when Grimgrin attacks, the last ability will be removed from the stack and have no effect.
**Status**: ISSUE

### Code issues

1. **ISSUE: Sacrifice ability auto-selects creature** (`grimgrin_corpse_born.rs:83-86`)
   - Oracle text says: `Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.`
   - Code does: `state.objects_in_zone(Zone::Battlefield, controller).iter().find(|o| o.id != object_id && o.power.is_some()).map(|o| o.id)` — auto-selects the first creature that isn't Grimgrin. The player should choose which creature to sacrifice.

2. **ISSUE: Sacrifice cost not declared in ActivatedAbilityDef** (`grimgrin_corpse_born.rs:69`)
   - Oracle text says: `Sacrifice another creature` (this is a cost)
   - Code does: `sacrifice_cost: SacrificeCost::None` — the sacrifice is handled inside `on_activate_ability` instead of being declared as a cost. This means the engine may show the ability as available even when the controller has no other creatures. Note: the engine's `SacrificeCost::SacrificeCreature` does not have an "another" exclusion, which may explain the manual approach, but the cost should still be enforced at the ability-availability level.

3. **ISSUE: Attack trigger auto-targets** (`grimgrin_corpse_born.rs:110-113`)
   - Oracle text says: `destroy target creature defending player controls`
   - Code does: `state.objects_in_zone(Zone::Battlefield, defender).iter().find(|o| o.power.is_some()).map(|o| o.id)` — auto-selects the first creature. The player should choose which creature to target.

4. **ISSUE: +1/+1 counter added even when no target exists** (`grimgrin_corpse_born.rs:123`)
   - Ruling says: `If the defending player controls no creatures when Grimgrin attacks, the last ability will be removed from the stack and have no effect.`
   - Code does: `state.add_counters(self_id, CounterType::PlusOnePlusOne, 1)` is unconditional (line 123). When no creatures exist on the defending side, the counter is still added. It should only be added when a valid target exists.

5. **ISSUE: Attack trigger uses `state.opponent()` instead of combat state** (`grimgrin_corpse_born.rs:109`)
   - Oracle text says: `destroy target creature defending player controls`
   - Code does: `let defender = state.opponent(controller)` — gets the opponent generically rather than consulting `state.combat.as_ref().and_then(|c| c.attackers.get(&self_id).copied())` which would correctly identify the defending player from the combat state. This works in 2-player games but is technically incorrect.

### Tricky interactions checked
- Enters tapped: PASS (line 51, `obj.tapped = true`)
- Doesn't untap during untap step: PASS (`PreventUntap { scope: EffectScope::OnSelf }`)
- Sacrifice untaps and adds counter: PASS (lines 92-96)
- +1/+1 when target survives (indestructible): PASS per code structure (counter is unconditional after destroy attempt)
- Fizzle when target becomes illegal: NOT IMPLEMENTED (engine limitation, targets are selected at resolution)
- No valid target = no effect: FAIL (counter still added when no creatures exist)

### Test coverage
- Enters tapped: `tier15_cards.rs:922` (grimgrin_enters_tapped)
- Sacrifice untaps and adds counter: `tier15_cards.rs:938` (grimgrin_sacrifice_untaps_and_counters)
- Attack trigger destroys creature: NOT TESTED
- Attack trigger +1/+1 counter: NOT TESTED
- No target = no effect (ruling 3): NOT TESTED
- Fizzle when target illegal (ruling 2): NOT TESTED
- +1/+1 counter when target survives indestructible (ruling 1): NOT TESTED

## Audit — 2026-04-01 21:58

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/214/grimgrin-corpse-born
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Mana cost**: {3}{U}{B}
**P/T**: 5/5
**Rulings**:
- [2013-07-01] If Grimgrin's last ability resolves, but the targeted creature isn't destroyed (perhaps because it regenerated or has indestructible), you'll still put a +1/+1 on Grimgrin.
- [2011-09-22] If the targeted creature is an illegal target by the time Grimgrin's last ability resolves, the entire ability doesn't resolve and none of its effects will occur. You won't put a +1/+1 counter on Grimgrin.
- [2011-09-22] If the defending player controls no creatures when Grimgrin attacks, the last ability will be removed from the stack and have no effect.
**Status**: PASS

### Code issues
No issues found.

All issues from the previous audit (2026-04-01 15:30) have been resolved:
- Sacrifice cost is now declared as `SacrificeCost::SacrificeAnotherCreature` (line 69), which the engine uses to correctly hide the ability when no other creatures are present.
- Attack trigger now uses `present_target_choice` (line 114) which presents the player with a choice when multiple targets exist, and auto-applies only when there is exactly one mandatory target.
- No-target case correctly returns early (line 108-109) with no +1/+1 counter added.
- Defending player is now resolved from combat state (lines 93-96) with a fallback to `state.opponent()`.
- The `DestroyThenCounter` effect handler (engine.rs:2145-2156) correctly uses `try_destroy` and unconditionally adds the +1/+1 counter afterward, matching the ruling that the counter is added even if destruction fails (indestructible/regenerate).

### Notes

1. **Engine-level limitation: sacrifice target auto-selection** — When activating Grimgrin's sacrifice ability with multiple other creatures, the engine auto-selects the first eligible creature (engine.rs:1544-1552, marked with `TODO: Present choice to player when there are multiple options`). This is an engine limitation affecting all `SacrificeAnotherCreature` costs, not specific to Grimgrin.

2. **Missing from LLM card knowledge** — Grimgrin is not listed in the card knowledge section of `mtg-player/src/llm.rs`. AI players will not have strategic guidance for this card.

3. **Ability can be activated when untapped** — Per community rulings, Grimgrin's sacrifice ability can be activated even when Grimgrin is already untapped (to grow via +1/+1 counters). The code correctly implements this with `requires_tap: false` (line 68).

### Tricky interactions checked
- Enters tapped: PASS (on_resolve sets `obj.tapped = true`, line 51)
- Doesn't untap during untap step: PASS (`PreventUntap { scope: EffectScope::OnSelf }`, verified engine untap step skips locked permanents at engine.rs:2440-2460)
- Sacrifice untaps and adds counter: PASS (on_activate_ability, lines 79-82)
- Sacrifice requires "another" creature: PASS (`SacrificeCost::SacrificeAnotherCreature` excludes self at engine.rs:342-344)
- Sacrifice ability available when untapped: PASS (`requires_tap: false`, line 68)
- +1/+1 when target survives (indestructible): PASS (`DestroyThenCounter` handler adds counter unconditionally after `try_destroy`, engine.rs:2150-2153)
- No valid target = no effect: PASS (returns early at line 108-109, no counter added)
- Attack trigger targets only defending player's creatures: PASS (line 99 filters by defender)
- Defending player resolved from combat state: PASS (lines 93-96)
- Mandatory targeting (no "you may"): PASS (`optional: false` at line 125)
- Fizzle when target becomes illegal: NOT DIRECTLY TESTABLE (engine resolves targeting at trigger time, not at resolution time — engine-level limitation)

### Test coverage
- Enters tapped: `tier15_cards.rs:1243` (grimgrin_enters_tapped)
- Sacrifice untaps and adds counter: `tier15_cards.rs:1259` (grimgrin_sacrifice_untaps_and_counters)
- Sacrifice not available without other creatures: `tier15_cards.rs:1288` (grimgrin_sacrifice_not_available_without_other_creatures)
- Attack trigger destroys and adds counter: `tier15_cards.rs:1304` (grimgrin_attack_trigger_destroys_and_adds_counter)
- Attack trigger presents choice with multiple targets: `tier15_cards.rs:1330` (grimgrin_attack_trigger_presents_choice_with_multiple_targets)
- No target = no effect (ruling 3): `tier15_cards.rs:1373` (grimgrin_attack_no_targets_no_counter)
- +1/+1 counter when target is indestructible (ruling 1): `tier15_cards.rs:1396` (grimgrin_attack_indestructible_target_still_gets_counter)
- Attack trigger targets defending player's creatures only: `tier15_cards.rs:1427` (grimgrin_attack_uses_defending_player_from_combat)
- Fizzle when target becomes illegal (ruling 2): NOT TESTED (engine-level limitation)
- Sacrifice ability when already untapped: NOT TESTED (could add a test for +1/+1 counter growth without needing to untap)

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/214/grimgrin-corpse-born
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Status**: PASS

### Code issues
No issues found. All card data and behavior match the oracle text.

**Card data verification:**
- Mana cost: Oracle `{3}{U}{B}` matches code `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Blue), ManaSymbol::Colored(Color::Black)])` (line 19-22)
- Card types: Oracle "Legendary Creature — Zombie Warrior" matches code `card_types: vec![CardType::Creature]`, `supertypes: vec![Supertype::Legendary]`, `subtypes: vec!["Zombie".into(), "Warrior".into()]` (lines 24-26)
- Power/toughness: Oracle 5/5 matches code `power: Some(5), toughness: Some(5)` (lines 27-28)
- Keywords: None in oracle, `keywords: vec![]` in code (line 29)
- Oracle text field: Matches verbatim (line 30)

**Behavior verification:**
- Enters tapped: Oracle says "enters tapped". Code sets `obj.tapped = true` in `on_resolve` (line 51). PASS.
- Doesn't untap during untap step: Oracle says "doesn't untap during your untap step". Code declares `ContinuousEffect::PreventUntap { scope: EffectScope::OnSelf }` (lines 32-35). PASS.
- Sacrifice another creature to untap and add counter: Oracle says "Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it." Code uses `sacrifice_cost: SacrificeCost::SacrificeAnotherCreature` (line 69) which correctly excludes self (verified at engine.rs:362-366). `on_activate_ability` sets `obj.tapped = false` and calls `state.add_counters(object_id, CounterType::PlusOnePlusOne, 1)` (lines 79-82). PASS.
- Attack trigger: Oracle says "Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin." Code declares `TriggerKind::Attacks` in `triggered_abilities` (line 40), `on_attacks` resolves the defending player from combat state (lines 94-96), collects defender's creatures as targets (lines 99-103), and uses `present_target_choice` with `PendingEffect::DestroyThenCounter` (lines 114-125). PASS.
- `try_destroy` used correctly: The `DestroyThenCounter` handler at engine.rs:2230 calls `crate::destruction::try_destroy`, matching the oracle "destroy" keyword. Counter is added unconditionally after destroy attempt (engine.rs:2233), matching ruling that counter is added even if target survives via indestructible/regeneration. PASS.
- No valid targets = no effect: Code returns early at lines 108-109 when `targets.is_empty()`, matching ruling. PASS.
- Mandatory targeting: `present_target_choice` called with `optional: false` (line 125), correct since oracle does not say "you may". PASS.
- Triggered abilities declaration: `triggered_abilities` includes `TriggerKind::Attacks` (line 40). PASS.

**Anti-patterns checked:**
- `try_destroy` for attack trigger: Correctly used (engine.rs:2230). PASS.
- Missing `triggered_abilities` declaration: Present (lines 38-43). PASS.
- Wrong scope for sacrifice: `SacrificeCost::SacrificeAnotherCreature` correctly scoped (line 69). PASS.
- Self-exclusion for "another creature": Handled by `SacrificeAnotherCreature` variant which excludes the source permanent (engine.rs:364-366). PASS.

### Tricky interactions checked
- Enters tapped: PASS
- Doesn't untap during untap step: PASS
- Sacrifice untaps and adds +1/+1 counter: PASS
- Sacrifice requires "another" creature (self-exclusion): PASS
- Sacrifice ability usable when already untapped (requires_tap: false): PASS
- Attack trigger targets only defending player's creatures: PASS
- Defending player resolved from combat state with fallback: PASS
- +1/+1 counter added even if target survives (indestructible): PASS
- No valid targets = no effect (no counter): PASS
- Mandatory targeting (not optional): PASS
- Fizzle when target becomes illegal: NOT DIRECTLY TESTABLE (engine resolves targeting at trigger time — engine-level limitation)
- Engine-level note: sacrifice target auto-selected when multiple creatures available (engine.rs:1564-1567 TODO comment). Not a card-level issue.
- Grimgrin not in LLM card knowledge (mtg-player/src/llm.rs). Informational only.

### Test coverage
- Enters tapped: `tier15_cards.rs:1243` (grimgrin_enters_tapped)
- Sacrifice untaps and adds counter: `tier15_cards.rs:1259` (grimgrin_sacrifice_untaps_and_counters)
- Sacrifice not available without other creatures: `tier15_cards.rs:1288` (grimgrin_sacrifice_not_available_without_other_creatures)
- Attack trigger destroys and adds counter: `tier15_cards.rs:1304` (grimgrin_attack_trigger_destroys_and_adds_counter)
- Attack trigger presents choice with multiple targets: `tier15_cards.rs:1330` (grimgrin_attack_trigger_presents_choice_with_multiple_targets)
- No target = no effect (ruling 3): `tier15_cards.rs:1373` (grimgrin_attack_no_targets_no_counter)
- +1/+1 counter when target is indestructible (ruling 1): `tier15_cards.rs:1396` (grimgrin_attack_indestructible_target_still_gets_counter)
- Attack trigger targets defending player only: `tier15_cards.rs:1427` (grimgrin_attack_uses_defending_player_from_combat)
- Fizzle when target becomes illegal (ruling 2): NOT TESTED (engine limitation)
- Sacrifice ability when already untapped: NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/214/grimgrin-corpse-born
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Status**: PASS

### Code issues

No behavioral issues found. One minor cosmetic note:

- **MINOR: Oracle text field uses full name instead of short name** — The Scryfall oracle text reads `"Grimgrin enters tapped..."` but the code's `oracle_text` field (line 29) reads `"Grimgrin, Corpse-Born enters tapped..."`. This is a display-only difference; the actual card behavior is unaffected. Current Scryfall oracle templating uses the short name "Grimgrin" on the card itself.

**Card data verification:**
- Name: `"Grimgrin, Corpse-Born"` (line 18). Correct.
- Mana cost: Oracle `{3}{U}{B}` matches code `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Blue), ManaSymbol::Colored(Color::Black)])` (lines 19-23). PASS.
- Card types: Oracle "Legendary Creature -- Zombie Warrior" matches code `card_types: vec![CardType::Creature]`, `supertypes: vec![Supertype::Legendary]`, `subtypes: vec!["Zombie".into(), "Warrior".into()]` (lines 24-26). PASS.
- Power/toughness: Oracle 5/5 matches code `power: Some(5), toughness: Some(5)` (lines 27-28). PASS.
- Keywords: None in oracle, `keywords: vec![]` in code (line 30). PASS.

**Behavior verification:**
- Enters tapped: Oracle says "enters tapped". Code `on_resolve` sets `obj.tapped = true` (line 51). PASS.
- Doesn't untap during untap step: Oracle says "doesn't untap during your untap step". Code declares `ContinuousEffect::PreventUntap { scope: EffectScope::OnSelf }` (lines 32-35). PASS.
- Sacrifice another creature to untap and add counter: Oracle says "Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it." Code uses `sacrifice_cost: SacrificeCost::SacrificeAnotherCreature` (line 69), `on_activate_ability` sets `obj.tapped = false` (line 80) and calls `state.add_counters(object_id, CounterType::PlusOnePlusOne, 1)` (line 82). PASS.
- Self-exclusion: `SacrificeCost::SacrificeAnotherCreature` excludes the source permanent (verified engine.rs:362-366). PASS.
- Sacrifice ability usable when untapped: `requires_tap: false` (line 71). Correct per rules -- the ability has no tap symbol. PASS.
- Attack trigger: Oracle says "Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin." Code declares `TriggerKind::Attacks` in `triggered_abilities` (line 40). `on_attacks` resolves the defending player from combat state (lines 94-96), collects defender's creatures (lines 99-103), uses `present_target_choice` with `PendingEffect::DestroyThenCounter` (lines 114-125). PASS.
- `try_destroy` used: `DestroyThenCounter` handler at engine.rs:2230 calls `crate::destruction::try_destroy`. Correct for oracle "destroy" wording. PASS.
- Counter added unconditionally after destroy attempt: engine.rs:2233 calls `state.add_counters`. Matches ruling: "If Grimgrin's last ability resolves, but the targeted creature isn't destroyed (perhaps because it regenerated or has indestructible), you'll still put a +1/+1 on Grimgrin." PASS.
- No valid targets = no effect: Code returns early at lines 108-109 when `targets.is_empty()`. Matches ruling: "If the defending player controls no creatures when Grimgrin attacks, the last ability will be removed from the stack and have no effect." PASS.
- Mandatory targeting: `present_target_choice` called with `optional: false` (line 125). Correct since oracle does not say "you may". PASS.

**Anti-patterns checked:**
- `try_destroy` for attack trigger: Correctly used (engine.rs:2230). PASS.
- Missing `triggered_abilities` declaration: Present with `TriggerKind::Attacks` (lines 38-43). PASS.
- Wrong scope for sacrifice: `SacrificeCost::SacrificeAnotherCreature` properly scoped (line 69). PASS.
- Self-exclusion for "another creature": Handled by `SacrificeAnotherCreature` variant. PASS.

### Tricky interactions checked
- Enters tapped: PASS
- Doesn't untap during untap step: PASS
- Sacrifice untaps and adds +1/+1 counter: PASS
- Sacrifice requires "another" creature (self-exclusion): PASS
- Sacrifice ability usable when already untapped (requires_tap: false): PASS
- Attack trigger targets only defending player's creatures: PASS
- Defending player resolved from combat state with fallback to opponent: PASS
- +1/+1 counter added even if target survives (indestructible/regenerate): PASS
- No valid targets = no effect and no counter: PASS
- Mandatory targeting (not optional): PASS
- Fizzle when target becomes illegal: NOT DIRECTLY TESTABLE (engine resolves targeting at trigger time, not at resolution -- engine-level limitation)
- Engine-level note: sacrifice target auto-selected when multiple creatures available (engine TODO). Not a card-level issue.
- Grimgrin not in LLM card knowledge (mtg-player/src/llm.rs). Informational only.

### Test coverage
8 tests in `mtg-engine/tests/tier15_cards.rs`:
- `grimgrin_enters_tapped` (line 1243): enters tapped and on battlefield
- `grimgrin_sacrifice_untaps_and_counters` (line 1259): sacrifice untaps, adds +1/+1, zombie goes to graveyard
- `grimgrin_sacrifice_not_available_without_other_creatures` (line 1288): ability hidden when no other creatures
- `grimgrin_attack_trigger_destroys_and_adds_counter` (line 1304): single target auto-applied, destroyed, counter added
- `grimgrin_attack_trigger_presents_choice_with_multiple_targets` (line 1330): choice presented with multiple defender creatures
- `grimgrin_attack_no_targets_no_counter` (line 1373): no defender creatures = no effect, no counter (ruling 3)
- `grimgrin_attack_indestructible_target_still_gets_counter` (line 1396): indestructible target survives but counter still added (ruling 1)
- `grimgrin_attack_uses_defending_player_from_combat` (line 1427): only defending player's creatures targetable, not controller's

**Not tested:**
- Fizzle when target becomes illegal before resolution (ruling 2) -- engine limitation
- Sacrifice ability activation when Grimgrin is already untapped (for +1/+1 growth only)
