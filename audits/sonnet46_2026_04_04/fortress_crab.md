## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: (none — vanilla creature)
**Type line**: Creature — Crab
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Mana cost {3}{U} matches code (`Generic(3)` + `Colored(Color::Blue)`): pass
- P/T 1/6 matches code (`power: Some(1)`, `toughness: Some(6)`): pass
- Card type Creature with no supertypes matches code (`card_types: vec![CardType::Creature]`, `supertypes: vec![]`): pass
- Subtype Crab matches code (`subtypes: vec!["Crab".into()]`): pass
- No keywords (vanilla creature) matches code (`keywords: vec![]`): pass
- No triggered abilities, continuous effects, or flashback cost (vanilla creature) matches code: pass

### Test coverage
- P/T 1/6: `mtg-engine/tests/innistrad_cards.rs:58` (test `fortress_crab_is_1_6` asserts `power == Some(1)` and `toughness == Some(6)`)
- Mana cost {3}{U}: NOT TESTED
- Card type Creature / subtype Crab: NOT TESTED
- No keywords: NOT TESTED
