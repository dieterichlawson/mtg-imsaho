## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: (none — vanilla creature)
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Mana cost {1}{B} encoded as `Generic(1)` + `Colored(Color::Black)`: pass
- Card types `[CardType::Creature]` with no supertypes: pass
- Subtype `["Zombie"]` matches type line "Creature — Zombie": pass
- P/T 2/2 encoded as `power: Some(2), toughness: Some(2)`: pass
- Oracle text is empty string, matching vanilla creature with no printed text: pass
- No keywords declared — correct, card has no keyword abilities: pass
- No continuous effects, triggered abilities, flashback cost, or additional cost — all correct for a vanilla creature: pass

### Test coverage
- Correct mana cost, types, P/T, and oracle text for Walking Corpse: NOT TESTED
