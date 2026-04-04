## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creature spells you cast cost {2} less to cast. Creatures you control get -1/-1.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Cost reduction only affects generic mana (per ruling): PASS
- -1/-1 is continuous static ability, not counters: PASS  
- Creatures with 1 toughness die immediately upon entering: PASS
- Cost reduction doesn't apply to flashback costs: PASS
- Multiple Heartless Summonings stack (-2/-2, {4} cost reduction): PASS
- X-cost creatures work correctly (choose X, then reduce by 2): PASS
- Alternative costs like Rooftop Storm bypass cost reduction entirely: PASS
- Effect continuously re-evaluates when creatures change controller: PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic -1/-1 P/T modification: `mtg-engine/tests/tier14_cards.rs:149`
- Cost reduction for creature spells: `mtg-engine/tests/tier14_cards.rs:166`  
- No cost reduction for non-creature spells: `mtg-engine/tests/tier14_cards.rs:189`
- Cost reduction only affects generic mana: NOT TESTED
- Creatures with 1 toughness dying immediately: NOT TESTED
- X-cost creature interactions: NOT TESTED
- Multiple Heartless Summonings stacking: NOT TESTED
- Alternative cost interactions: NOT TESTED