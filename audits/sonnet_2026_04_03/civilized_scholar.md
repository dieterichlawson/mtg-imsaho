## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: 
Front face - {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
Back face - At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Status**: ISSUE

### Code issues

- Incorrect creature type detection in discard check (civilized_scholar.rs:113)
  - Oracle text says: `If a creature card is discarded this way, untap this creature, then transform it.`
  - Code does: `let is_creature = state.get_object(discard_id).map(|o| o.power.is_some()).unwrap_or(false);`
  - Should use `registry.card_data(card_id).map(|d| d.card_types.contains(&CardType::Creature)).unwrap_or(false)` to properly detect creature cards

- Missing Transform keyword (civilized_scholar.rs:34)
  - Oracle text says: `Keywords: Transform` 
  - Code does: `keywords: vec![]` (empty vector)
  - Should include Transform keyword in the keywords vector

### Tricky interactions checked

- "If a creature card is discarded this way" condition: FAIL - uses power check instead of card type check
- Attack tracking across transform faces: PASS - correctly uses card_state to track attacks
- Transform priority timing: PASS - implements immediate transform without priority window
- End step trigger only fires for controller: PASS - checks `state.active_player != controller`
- Triggered ability declarations: PASS - works despite unusual structure (front face declares EndStep for back face behavior)

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Transform on creature discard: `mtg-engine/tests/tier15_cards.rs:2760`
- No transform on non-creature discard: `mtg-engine/tests/tier15_cards.rs:2801`  
- Attack tracking across faces: NOT TESTED
- End step transform back behavior: NOT TESTED
- Priority timing during transform: NOT TESTED
- Multiple attackers with same creature: NOT TESTED