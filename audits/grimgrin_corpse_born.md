## Audit — 2026-04-01

**Scryfall Oracle text**: Grimgrin, Corpse-Born enters the battlefield tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Scryfall type line**: Legendary Creature — Zombie Warrior
**Status**: ISSUE

- Mana cost {3}{U}{B}: correct
- 5/5 stats: correct
- Supertype Legendary: correct
- Subtypes Zombie Warrior: correct
- Enters tapped: correct
- PreventUntap continuous effect: correct
- Sacrifice ability (untap + counter): correct in effect, but the sacrifice is auto-picked (first available creature) rather than player choice
- ISSUE: The sacrifice ability uses SacrificeCost::None and manually picks a creature to sacrifice in on_activate_ability. This means the engine doesn't enforce the sacrifice as a cost — the ability could be activated even with no other creatures. The auto-pick of "first available" creature is also not player-controlled.
- Attack trigger (destroy + counter): correctly destroys a creature the defending player controls and adds +1/+1 counter. Target auto-picked as first found creature rather than player choice.
- Tests exist in tier15_cards.rs covering enters-tapped, sacrifice/untap/counter mechanics

## Audit — 2026-04-01

**Scryfall Oracle text**: Grimgrin, Corpse-Born enters tapped and doesn't untap during your untap step. / Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it. / Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Scryfall type line**: Legendary Creature — Zombie Warrior
**Status**: ISSUE

1. **Sacrifice ability auto-picks creature**: The sacrifice activated ability auto-picks the first available creature to sacrifice instead of presenting a choice to the player. (Line 83-86 in grimgrin_corpse_born.rs)
2. **Sacrifice ability doesn't use SacrificeCost**: The sacrifice cost should be modeled as part of the ability cost (SacrificeCost::SacrificeAnother or similar), not handled manually in on_activate_ability. Currently SacrificeCost is set to None. (Line 69)
3. **Attack trigger auto-picks target**: Oracle says "destroy target creature defending player controls" — the code auto-picks the first creature instead of presenting target choice. (Line 110-113)
4. **Attack trigger: +1/+1 counter always added**: Per ruling, if the targeted creature is illegal when the ability resolves, the entire ability doesn't resolve and no counter is added. The code always adds the counter regardless of whether the destroy succeeded or target was legal. (Line 122-123)
5. **Oracle says "then"**: The "destroy... then put a +1/+1 counter" means both happen in sequence as part of one resolution. If target is illegal, entire ability fizzles. Code doesn't check this.
