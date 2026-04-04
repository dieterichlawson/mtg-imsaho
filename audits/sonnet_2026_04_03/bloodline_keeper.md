## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: 
Front face - Bloodline Keeper {2}{B}{B} 3/3 Creature — Vampire: Flying, {T}: Create a 2/2 black Vampire creature token with flying, {B}: Transform this creature. Activate only if you control five or more Vampires.
Back face - Lord of Lineage 5/5 Creature — Vampire: Flying, Other Vampire creatures you control get +2/+2, {T}: Create a 2/2 black Vampire creature token with flying.
**Type line**: Creature — Vampire (both faces)
**Status**: ISSUE

### Code issues
- Transform keyword missing from front face keywords
  - Oracle text says: `Keywords: Flying, Transform`
  - Code does: `keywords: vec![Keyword::Flying]` - only includes Flying, missing Transform

### Tricky interactions checked
- Self-counting for vampire requirement: PASS - Bloodline Keeper counts itself toward the 5 vampire requirement correctly
- Vampire counting includes both registry and token subtypes: PASS - count_vampires function checks both `o.subtypes` and `registry.card_data()`
- Transform ability doesn't re-check vampire count on resolution: PASS - once on stack, transforms regardless of vampire count changes
- Continuous effect excludes self on back face: PASS - uses EffectScope::GlobalOther which properly excludes the source
- Token creation works for both faces: PASS - both faces have ability_index 0 for token creation
- Dynamic P/T correctly returns 5/5 when transformed: PASS - dynamic_pt checks is_transformed flag
- Can activate tap and transform abilities independently: PASS - transform ability doesn't require tap

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Token creation from Bloodline Keeper: `tier15_cards.rs:1429` 
- Transform ability activation: NOT TESTED
- Vampire counting logic (5+ requirement): NOT TESTED
- Continuous effect (+2/+2 to other vampires): NOT TESTED
- Self-counting in vampire requirement: NOT TESTED
- Vampire count check timing (activation vs resolution): NOT TESTED
- Back face token creation: NOT TESTED

Sources:
- [Bloodline Keeper // Lord of Lineage · Innistrad (ISD) #90](https://scryfall.com/card/isd/90/bloodline-keeper-lord-of-lineage)
- [Bloodline Keeper transforming into Lord of Lineage faq rulings](https://rarityguide.com/forums/magic-rules-questions/15686520-bloodline-keeper-transforming-into-lord-lineage-faq-rulings.html)
- [Transforming Bloodline - Magic Rulings Archives](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/magic-rulings-archives/299732-transforming-bloodline)