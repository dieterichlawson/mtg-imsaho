## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues
- engine.rs lines 836-860, 1074-1090, 1343-1357: AnyTarget implementation missing planeswalker support
  - Oracle text says: `Devil's Play deals X damage to any target`
  - Code does: Only allows targeting creatures (`obj.power.is_some()`) and players, but excludes planeswalkers. In MTG, "any target" includes creatures, players, AND planeswalkers per 2018 rules update that removed planeswalker damage redirect rule.

### Tricky interactions checked
- X=0 behavior: PASS - spell resolves but deals no damage, correctly moves to appropriate zone
- Flashback exile mechanism: PASS - cast_with_flashback flag set correctly, move_spell_after_resolve exiles flashback spells
- X calculation with flashback cost: PASS - engine correctly calculates X from remaining mana after paying {R}{R}{R} colored requirements
- "Any target" scope: FAIL - missing planeswalkers (engine bug affects all AnyTarget spells, not just Devil's Play)
- Optional flashback casting: PASS - flashback offered as optional CastSpell action when card in graveyard with sufficient mana
- Spell cleanup after resolution: PASS - correctly uses move_spell_after_resolve helper

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic X damage dealing: `tier14_cards.rs:298-314` 
- X=0 edge case: `tier14_cards.rs:318-331`
- Flashback mechanism for Devil's Play: NOT TESTED
- Planeswalker targeting: NOT TESTED (impossible due to engine AnyTarget bug)
- X-cost calculation with flashback: NOT TESTED
- CMC vs mana paid distinction: NOT TESTED
- Flashback timing restrictions: NOT TESTED for Devil's Play specifically
- Exile after flashback resolution: NOT TESTED for Devil's Play specifically