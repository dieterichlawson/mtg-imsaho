## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: (empty — vanilla creature)
**Type line**: Creature — Dog
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Mana cost {4}{W}: code uses `Generic(4)` + `Colored(Color::White)` — matches oracle exactly.
- P/T 3/5: code sets `power: Some(3)`, `toughness: Some(5)` — matches oracle exactly.
- Subtype "Dog": code sets `subtypes: vec!["Dog".into()]` — matches oracle exactly.
- Empty oracle text / no abilities: code sets `oracle_text: String::new()`, `keywords: vec![]`, `triggered_abilities: vec![]`, `continuous_effects: vec![]` — correct for a vanilla creature.

### Test coverage
- Mana cost, P/T, subtype, vanilla body: NOT TESTED
