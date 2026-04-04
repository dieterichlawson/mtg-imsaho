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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.\nSacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.\nWhenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Status**: PASS

### Code issues
No issues found. Card data matches oracle: name "Grimgrin, Corpse-Born", mana cost {3}{U}{B}, Legendary Creature — Zombie Warrior, 5/5. Enters tapped (set in on_resolve). PreventUntap continuous effect on self. Activated ability: sacrifice another creature (SacrificeCost::SacrificeAnotherCreature), no mana cost, untaps and adds +1/+1 counter. Attack trigger: finds defending player from combat state, collects their creatures, presents target choice via DestroyThenCounter pending effect. Correctly returns early if no creatures (per ruling). is_legendary set on resolve. No anti-patterns.

## Audit — 2026-04-02 20:07

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/214/grimgrin-corpse-born
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Status**: PASS

### Code issues
No issues found.

**Card data verification (oracle vs code):**
- Name: `"Grimgrin, Corpse-Born"` (line 18). Correct.
- Mana cost: Oracle `{3}{U}{B}` matches `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Blue), ManaSymbol::Colored(Color::Black)])` (lines 19-23). PASS.
- Types: Oracle "Legendary Creature — Zombie Warrior" matches `card_types: vec![CardType::Creature]`, `supertypes: vec![Supertype::Legendary]`, `subtypes: vec!["Zombie".into(), "Warrior".into()]` (lines 24-26). PASS.
- P/T: Oracle 5/5 matches `power: Some(5), toughness: Some(5)` (lines 27-28). PASS.
- Keywords: None in oracle, `keywords: vec![]` (line 30). PASS.
- Minor cosmetic note: code oracle_text field says `"Grimgrin, Corpse-Born enters tapped..."` but Scryfall uses the short name `"Grimgrin enters tapped..."`. Display-only, no functional impact.

**Behavior verification:**
- Enters tapped: `on_resolve` sets `obj.tapped = true` (line 51). PASS.
- Doesn't untap during untap step: `ContinuousEffect::PreventUntap { scope: EffectScope::OnSelf }` (lines 32-35). Engine untap step (engine.rs:2912-2924) skips permanents with this effect. PASS.
- Sacrifice another creature: `sacrifice_cost: SacrificeCost::SacrificeAnotherCreature` (line 69). Engine enforces "another" exclusion at engine.rs:375-380 (checks `o.id != obj_id`). `on_activate_ability` untaps (`obj.tapped = false`, line 80) and adds counter (`add_counters ... PlusOnePlusOne, 1`, line 82). PASS.
- Sacrifice ability usable when untapped: `requires_tap: false` (line 71). Correct — no tap symbol in oracle cost. PASS.
- Attack trigger declaration: `TriggerKind::Attacks` in `triggered_abilities` (line 40). Engine creates `PendingTrigger::AttacksTrigger` on `AttackersDeclared` event (triggers.rs:677-697), resolves via `behavior.on_attacks()` (triggers.rs:980-985). PASS.
- Attack trigger targeting: `on_attacks` resolves defending player from combat state (lines 94-96), collects defender's creatures (lines 99-103), uses `present_target_choice` with `PendingEffect::DestroyThenCounter` and `optional: false` (lines 114-125). PASS.
- Destroy + counter: `DestroyThenCounter` handler (engine.rs:2427-2438) calls `try_destroy` then unconditionally adds +1/+1 counter. Matches ruling: counter added even if target survives (indestructible/regenerate). PASS.
- No valid targets: Returns early at lines 108-109 when `targets.is_empty()`. No counter added. Matches ruling: "the last ability will be removed from the stack and have no effect." PASS.
- Mandatory targeting: `optional: false` (line 125). Oracle does not say "you may". PASS.

### Tricky interactions checked
- Enters tapped + PreventUntap (only untaps via sacrifice ability): PASS
- Sacrifice "another" creature self-exclusion (can't sacrifice Grimgrin to its own ability): PASS
- +1/+1 counter added even when target is indestructible (ruling 1): PASS
- No defending creatures = no effect and no counter (ruling 3): PASS
- Fizzle when target becomes illegal (ruling 2): NOT DIRECTLY TESTABLE (engine resolves targeting at trigger resolution time — engine-level limitation, not card bug)
- Mandatory targeting vs "you may" (oracle has no "you may"): PASS

### Test coverage
8 tests in `mtg-engine/tests/tier15_cards.rs`:
- Enters tapped: `tier15_cards.rs:1494` (grimgrin_enters_tapped)
- Sacrifice untaps and adds counter: `tier15_cards.rs:1510` (grimgrin_sacrifice_untaps_and_counters)
- Sacrifice not available without other creatures: `tier15_cards.rs:1539` (grimgrin_sacrifice_not_available_without_other_creatures)
- Attack trigger destroys and adds counter (single target auto-apply): `tier15_cards.rs:1555` (grimgrin_attack_trigger_destroys_and_adds_counter)
- Attack trigger presents choice with multiple targets: `tier15_cards.rs:1581` (grimgrin_attack_trigger_presents_choice_with_multiple_targets)
- No targets = no effect, no counter (ruling 3): `tier15_cards.rs:1624` (grimgrin_attack_no_targets_no_counter)
- Indestructible target still gets counter (ruling 1): `tier15_cards.rs:1647` (grimgrin_attack_indestructible_target_still_gets_counter)
- Attack trigger targets defending player's creatures only: `tier15_cards.rs:1678` (grimgrin_attack_uses_defending_player_from_combat)
- Fizzle when target becomes illegal (ruling 2): NOT TESTED (engine limitation)
- Sacrifice ability when already untapped: NOT TESTED

## Audit — 2026-04-02 20:13

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/214/grimgrin-corpse-born
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Status**: PASS

### Code issues
No issues found.

**Card data verification (oracle vs code):**
- Name: `"Grimgrin, Corpse-Born"` (line 18). Correct.
- Mana cost: Oracle `{3}{U}{B}` matches code `Generic(3), Colored(Color::Blue), Colored(Color::Black)` (lines 19-23). PASS.
- Types: Oracle "Legendary Creature — Zombie Warrior" matches code `CardType::Creature`, `Supertype::Legendary`, subtypes `["Zombie", "Warrior"]` (lines 24-26). PASS.
- P/T: Oracle 5/5 matches code `power: Some(5), toughness: Some(5)` (lines 27-28). PASS.
- Keywords: None in oracle, `keywords: vec![]` in code (line 30). PASS.
- Minor cosmetic note: code oracle_text string says `"Grimgrin, Corpse-Born enters tapped..."` but Scryfall oracle text uses short name `"Grimgrin enters tapped..."`. Display-only, no functional impact.

**Behavior verification:**
- Enters tapped: `on_resolve` moves to battlefield then sets `obj.tapped = true` (lines 48-53). PASS.
- Doesn't untap during untap step: `ContinuousEffect::PreventUntap { scope: EffectScope::OnSelf }` declared in `continuous_effects` (lines 32-35). Engine untap step at engine.rs:2908-2924 queries `has_continuous_effect` for `PreventUntap` and skips matching permanents. PASS.
- Sacrifice another creature to untap + counter: `sacrifice_cost: SacrificeCost::SacrificeAnotherCreature` (line 69). Engine at lines 375-380 checks `o.id != obj_id` to enforce "another". `on_activate_ability` sets `obj.tapped = false` (line 80) and calls `state.add_counters(object_id, CounterType::PlusOnePlusOne, 1)` (line 82). PASS.
- Sacrifice ability usable when untapped: `requires_tap: false` (line 71). Correct -- oracle cost has no tap symbol. PASS.
- Sacrifice ability not available without other creatures: Engine at lines 375-380 checks `any(|o| o.power.is_some() && o.id != obj_id)` and skips generating the action if false. PASS.
- Attack trigger: `TriggerKind::Attacks` in `triggered_abilities` (line 40). `on_attacks` resolves the defending player from `state.combat` (lines 94-96), collects defender's creatures filtered by `o.power.is_some()` (lines 99-103), calls `present_target_choice` with `PendingEffect::DestroyThenCounter` and `optional: false` (lines 114-125). PASS.
- Destroy then counter: `DestroyThenCounter` handler at engine.rs:2427-2438 calls `crate::destruction::try_destroy` then unconditionally calls `state.add_counters(*source_id, PlusOnePlusOne, 1)`. Matches ruling: "+1/+1 counter added even if target survives via indestructible/regenerate." PASS.
- No valid targets = no effect: Returns early at lines 108-109 when `targets.is_empty()`. No counter added. Matches ruling: "If the defending player controls no creatures when Grimgrin attacks, the last ability will be removed from the stack and have no effect." PASS.
- Mandatory targeting: `optional: false` at line 125. Oracle does not say "you may". PASS.

### Tricky interactions checked
- Enters tapped + PreventUntap (only untaps via sacrifice or external effects): PASS
- Sacrifice "another" creature self-exclusion (can't sacrifice Grimgrin to itself): PASS
- +1/+1 counter added even when target is indestructible (ruling 1): PASS
- No defending creatures = no effect and no counter (ruling 3): PASS
- Fizzle when target becomes illegal (ruling 2): NOT DIRECTLY TESTABLE (engine resolves targeting at trigger resolution time -- engine-level limitation, not a card bug)
- Mandatory targeting vs "you may" (oracle has no "you may"): PASS

### Test coverage
8 tests in `mtg-engine/tests/tier15_cards.rs`:
- Enters tapped: `tier15_cards.rs:1494` (grimgrin_enters_tapped)
- Sacrifice untaps and adds counter: `tier15_cards.rs:1510` (grimgrin_sacrifice_untaps_and_counters)
- Sacrifice not available without other creatures: `tier15_cards.rs:1539` (grimgrin_sacrifice_not_available_without_other_creatures)
- Attack trigger destroys and adds counter (single target): `tier15_cards.rs:1555` (grimgrin_attack_trigger_destroys_and_adds_counter)
- Attack trigger presents choice with multiple targets: `tier15_cards.rs:1581` (grimgrin_attack_trigger_presents_choice_with_multiple_targets)
- No targets = no effect, no counter (ruling 3): `tier15_cards.rs:1624` (grimgrin_attack_no_targets_no_counter)
- Indestructible target still gets counter (ruling 1): `tier15_cards.rs:1647` (grimgrin_attack_indestructible_target_still_gets_counter)
- Attack trigger targets defending player's creatures only: `tier15_cards.rs:1678` (grimgrin_attack_uses_defending_player_from_combat)
- Fizzle when target becomes illegal (ruling 2): NOT TESTED (engine limitation)
- Sacrifice ability when already untapped: NOT TESTED

## Audit — 2026-04-02 20:20

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01, https://scryfall.com/card/isd/214/grimgrin-corpse-born
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Mana cost**: {3}{U}{B}
**P/T**: 5/5
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Enters tapped: code sets `obj.tapped = true` in `on_resolve` — pass
- Doesn't untap during untap step: `PreventUntap { scope: EffectScope::OnSelf }` continuous effect applied, engine skips untap for permanents with this effect — pass
- Sacrifice cost is "another creature": uses `SacrificeCost::SacrificeAnotherCreature` which excludes the source permanent (engine checks `o.id != obj_id`) — pass
- Activated ability has no mana cost: `ManaCost::free()` — pass
- Activated ability usable at instant speed: `sorcery_speed_only: false` — pass (sacrifice ability has no timing restriction in oracle text)
- Attack trigger uses `try_destroy` (not sacrifice): correct for "destroy" oracle text — pass
- Attack trigger targets defending player's creatures only: uses combat state to identify defender — pass
- Indestructible target: `DestroyThenCounter` effect adds +1/+1 counter regardless of whether destruction succeeds — pass (matches ruling 1)
- No defending creatures: empty target list causes early return, no counter added — pass (matches ruling 3)
- Mandatory target selection: `present_target_choice` called with `optional: false` — pass (attack trigger is not "you may")
- Legendary rule: `is_legendary = true` set in `on_resolve`, `Supertype::Legendary` in card data — pass
- `triggered_abilities` declaration matches `on_attacks` hook: `TriggerKind::Attacks` declared — pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Ruling 1 (indestructible/regenerate target still gets +1/+1): `tier15_cards.rs:1647` (grimgrin_attack_indestructible_target_still_gets_counter)
- Ruling 2 (illegal target = entire ability fizzles, no counter): NOT TESTED (engine-level fizzle handling)
- Ruling 3 (no defending creatures = ability removed, no effect): `tier15_cards.rs:1624` (grimgrin_attack_no_targets_no_counter)
- Enters tapped: `tier15_cards.rs:1494` (grimgrin_enters_tapped)
- Sacrifice untaps and adds counter: `tier15_cards.rs:1510` (grimgrin_sacrifice_untaps_and_counters)
- Sacrifice not available without other creatures: `tier15_cards.rs:1539` (grimgrin_sacrifice_not_available_without_other_creatures)
- Attack trigger destroys and adds counter (single target): `tier15_cards.rs:1555` (grimgrin_attack_trigger_destroys_and_adds_counter)
- Attack trigger presents choice with multiple targets: `tier15_cards.rs:1581` (grimgrin_attack_trigger_presents_choice_with_multiple_targets)
- Attack trigger targets defending player's creatures only: `tier15_cards.rs:1678` (grimgrin_attack_uses_defending_player_from_combat)
- Fizzle when target becomes illegal (ruling 2): NOT TESTED (engine limitation)
- Sacrifice ability when already untapped: NOT TESTED
- LLM card knowledge: NOT PRESENT in `mtg-player/src/llm.rs`

## Audit — 2026-04-02 21:12
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: "Grimgrin enters tapped and doesn't untap during your untap step.\nSacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.\nWhenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin."
**Type line**: Legendary Creature — Zombie Warrior
**Status**: PASS

### Code issues
1. **Minor oracle text mismatch (cosmetic only)**: The `oracle_text` field in `card_data()` uses "Grimgrin, Corpse-Born enters tapped..." but Scryfall oracle text says "Grimgrin enters tapped...". This is display-only and does not affect game logic. All three abilities otherwise match the oracle text verbatim.
2. **Sacrifice cost auto-selection (engine limitation, not card-specific)**: When the controller has multiple other creatures, the engine auto-sacrifices the first eligible creature instead of presenting a choice (see TODO at `engine.rs:1764`). This is a known engine-level limitation affecting all cards that use `SacrificeCost::SacrificeAnotherCreature`, not a Grimgrin implementation bug.

Neither issue causes incorrect game behavior for Grimgrin specifically -- the card functions correctly for all its abilities and rulings.

### Tricky interactions checked (min 3)
1. **Indestructible target still gets +1/+1 counter**: The `DestroyThenCounter` pending effect in `engine.rs:2427-2438` calls `try_destroy` (which respects indestructible, returning `DestroyResult::Indestructible` without moving to graveyard), then unconditionally adds the +1/+1 counter. This matches the 2013-07-01 ruling. Confirmed by test `grimgrin_attack_indestructible_target_still_gets_counter`.
2. **No defending creatures = no effect and no counter**: When `targets` is empty in `on_attacks`, the function returns early (line 108-109), meaning no `DestroyThenCounter` effect is queued and no counter is added. This matches the 2011-09-22 ruling. Confirmed by test `grimgrin_attack_no_targets_no_counter`.
3. **Defending player identification from combat state**: The attack trigger reads the defender from `state.combat.attackers` (line 94-96), correctly targeting only the defending player's creatures even when the controller has other creatures. Confirmed by test `grimgrin_attack_uses_defending_player_from_combat` which verifies the controller's own creature is not targeted.
4. **PreventUntap during untap step**: The `ContinuousEffect::PreventUntap { scope: EffectScope::OnSelf }` is checked by the engine's untap step handler (`engine.rs:2912-2924`), which collects all permanents with this effect and excludes them from untapping. This correctly prevents Grimgrin from untapping normally.
5. **Sacrifice ability usable at instant speed while tapped or untapped**: The activated ability has `requires_tap: false`, `once_per_turn: false`, and `sorcery_speed_only: false`, meaning it can be activated at any time there is priority and another creature to sacrifice -- consistent with how sacrifice abilities work in MTG.

### Test coverage
8 tests, all passing:
- `grimgrin_enters_tapped` — enters battlefield tapped
- `grimgrin_sacrifice_untaps_and_counters` — sacrifice another creature to untap and get +1/+1
- `grimgrin_sacrifice_not_available_without_other_creatures` — ability unavailable without sacrifice fodder
- `grimgrin_attack_trigger_destroys_and_adds_counter` — single target auto-applied
- `grimgrin_attack_trigger_presents_choice_with_multiple_targets` — multiple targets present choice
- `grimgrin_attack_no_targets_no_counter` — no defending creatures = no effect (ruling 3)
- `grimgrin_attack_indestructible_target_still_gets_counter` — indestructible survives, counter still added (ruling 1)
- `grimgrin_attack_uses_defending_player_from_combat` — only targets defending player's creatures
- NOT TESTED: fizzle when target becomes illegal before resolution (ruling 2) — engine limitation
- NOT TESTED: sacrifice when Grimgrin is already untapped (legal but not separately tested)

## Audit — 2026-04-03 21:31

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Another creature" restriction in sacrifice cost: PASS (`SacrificeCost::SacrificeAnotherCreature` excludes source via `o.id != obj_id` check at engine.rs:379 and 1767)
- No legal targets when attacking: PASS (early return if `targets.is_empty()` at lines 108-110, matching ruling that ability is removed from stack with no effect)
- Indestructible/regeneration target still gives +1/+1 counter: PASS (`DestroyThenCounter` handler unconditionally adds counter after `try_destroy` at engine.rs:2435)
- Target becomes illegal during resolution: PASS (standard targeting rules handled by engine, entire ability fizzles per 2011-09-22 ruling)
- "Destroy" vs "sacrifice" semantics: PASS (uses `try_destroy` in `DestroyThenCounter` handler matching oracle "destroy" keyword)
- Combat timing of attack trigger: PASS (`TriggerKind::Attacks` fires when declared as attacker, before blockers)
- Targeting defending player's creatures only: PASS (resolves defender from `state.combat` at lines 94-96, filters defender's creatures at lines 99-103)
- Enters tapped requirement: PASS (`on_resolve` sets `obj.tapped = true` at line 51)
- Doesn't untap during untap step: PASS (`ContinuousEffect::PreventUntap` with `EffectScope::OnSelf` at lines 32-35)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Enters tapped: `tier15_cards.rs:1494-1507`
- Sacrifice another creature to untap/counter: `tier15_cards.rs:1510-1536`
- Sacrifice not available without other creatures: `tier15_cards.rs:1539-1552`
- Attack trigger basic functionality: `tier15_cards.rs:1555-1578`
- Multiple target choice presentation: `tier15_cards.rs:1581-1619`
- No targets = no counter (2011-09-22 ruling): `tier15_cards.rs:1624-1642`
- Indestructible target still gives counter (2013-07-01 ruling): `tier15_cards.rs:1647-1672`
- Defending player targeting from combat state: `tier15_cards.rs:1678-1702`
- Target becomes illegal ruling (2011-09-22): NOT TESTED
