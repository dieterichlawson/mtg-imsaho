## Audit — 2026-04-01

**Scryfall Oracle text**: Stromkirk Noble can't be blocked by Humans.\nWhenever Stromkirk Noble deals combat damage to a player, put a +1/+1 counter on it.
**Scryfall type line**: Creature — Vampire Noble
**Status**: PASS

- Name: correct ("Stromkirk Noble")
- Cost: {R} -- correct
- Type: Creature -- correct
- Subtypes: Vampire, Noble -- correct
- P/T: 1/1 -- correct
- Block restriction: implemented via ContinuousEffect::BlockRestriction with CreatureFilter::Not(HasSubtype("Human")) -- correct
- Combat damage trigger: adds +1/+1 counter via `on_combat_damage_to_player` -- correct
- Tests exist in `tier6_cards.rs` covering both the block restriction and the counter trigger
- No issues found
