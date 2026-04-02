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
