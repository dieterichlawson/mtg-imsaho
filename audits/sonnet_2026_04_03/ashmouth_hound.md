## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
**Type line**: Creature — Elemental Dog
**Status**: PASS

### Code issues

No issues found.

### Tricky interactions checked

- **"blocks or becomes blocked"**: PASS - Two separate triggers (TriggerKind::Blocks and TriggerKind::BecomesBlocked) correctly handle both scenarios
- **"this creature deals 1 damage to that creature"**: PASS - Correct targeting (blocked_attacker for blocks, blocker_id for becomes_blocked)  
- **"once for each creature" ruling**: PASS - Trigger dispatch creates one trigger per blocker/attacker pair in assignments
- **Non-combat damage timing**: PASS - Uses NonCombatDamageDealt event, resolves before combat damage step
- **Zone checks on trigger resolution**: PASS - Both triggers verify source is on battlefield before resolving
- **Target validation**: PASS - deal_1_damage function checks target is on battlefield before applying damage
- **Multiple blockers/blocked creatures**: PASS - Loop over assignments creates correct number of triggers
- **Source leaves battlefield**: PASS - Trigger resolution checks object_id zone before executing

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- **Blocks trigger**: `mtg-engine/tests/tier12_cards.rs:169` (ashmouth_hound_deals_damage_on_block)
- **Becomes blocked trigger**: NOT TESTED
- **Multiple creatures interaction**: NOT TESTED  
- **Timing vs combat damage**: NOT TESTED
- **Target leaving battlefield**: NOT TESTED
- **Source leaving battlefield**: NOT TESTED