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
