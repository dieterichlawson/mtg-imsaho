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
