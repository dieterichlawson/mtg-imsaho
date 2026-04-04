## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
Creatures enchanted player controls attack each combat if able.
**Type line**: Enchantment — Aura Curse
**Status**: ISSUE

### Code issues

- Attack target auto-selected instead of player choice (mtg-engine/src/engine.rs:1857)
  - Oracle text says: `The enchanted player still chooses which player or planeswalker each creature they control attacks.`
  - Code does: `combat.attackers.insert(*id, defending);` where `defending = new_state.opponent(new_state.active_player)` - auto-assigns target to opponent without player choice

- Forced attack ignores "can't attack" effects (mtg-engine/src/engine.rs:1838-1844)  
  - Oracle text says: `attack each combat if able` and ruling states `If...a creature they control is tapped, is affected by a spell or ability that says it can't attack...then it doesn't attack`
  - Code does: Only checks for tapped, summoning sickness, and defender, but never calls `state.can_attack()` to check for `PreventAttack` continuous effects (e.g., from Pacifism, non-Human Bonds of Faith)

### Tricky interactions checked
- "if able" clause with summoning sickness: pass (line 1827 checks creature.summoning_sick)
- "if able" clause with tapped state: pass (line 1827 checks creature.tapped)  
- "if able" clause with Defender keyword: pass (lines 1834-1836 skip creatures with Defender)
- "if able" clause with PreventAttack effects: fail (never calls can_attack() to check PreventAttack continuous effects)
- Player choice of attack target: fail (auto-assigns to opponent instead of presenting choice)
- Effect scope limited to attached player's creatures: pass (EffectScope::Global(CreatureFilter::AttachedPlayer) correctly implemented)
- Vigilance interaction with forced attack: pass (lines 1864-1872 check for vigilance before tapping)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic forced attack functionality: `mtg-engine/tests/tier7_cards.rs:323-353`
- Effect only applies to enchanted player's creatures: `mtg-engine/tests/tier7_cards.rs:344-352`
- "if able" clause with summoning sickness: NOT TESTED
- "if able" clause with tapped creatures: NOT TESTED
- "if able" clause with Defender creatures: NOT TESTED  
- "if able" clause with PreventAttack effects: NOT TESTED
- Player choice of attack targets in multiplayer: NOT TESTED
- Interaction with attack cost effects: NOT TESTED
- Vigilance preventing tap from forced attack: NOT TESTED