## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "may" optionality: PASS - Lines 66-73 present ResolutionChoiceKind::YesNo, allowing player to decline
- "to an opponent" targeting: PASS - Lines 61-64 correctly exclude damage to Curiosity's controller, trigger on damage to any other player
- "enchanted creature" source check: PASS - Lines 57-59 verify source_id matches attached_to
- Any damage type (combat vs non-combat): PASS - TriggerKind::AnyDamageToPlayer fires on both CombatDamageDealt and NonCombatDamageDealt events (triggers.rs:543-559, 576-594)
- Planeswalker damage exclusion: PASS - Engine only dispatches AnyDamageToPlayer for DamageTarget::Player, not DamageTarget::Object, so planeswalker damage won't trigger
- One card per damage event: PASS - Each trigger resolution calls draw_cards(state, controller, 1) exactly once
- Controller vs owner distinction: PASS - Uses aura.controller (line 61) as required by ruling that "you" refers to Curiosity's controller
- Aura leaves battlefield: PASS - Line 49-50 check aura is still on battlefield before processing trigger

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic trigger on combat damage: tests/tier6_cards.rs:357
- Player choice "may" (accepting): tests/tier6_cards.rs:357
- Player choice "may" (declining): tests/tier6_cards.rs:405
- Non-combat damage triggering: NOT TESTED
- Planeswalker damage not triggering: NOT TESTED
- Multiple damage events in one turn: NOT TESTED
- Aura on opponent's creature not triggering when it damages you: NOT TESTED
- Aura removed between trigger and resolution: NOT TESTED