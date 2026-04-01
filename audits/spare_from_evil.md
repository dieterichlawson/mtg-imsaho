## Audit — 2026-04-01

**Scryfall Oracle text**: Creatures you control gain protection from non-Human creatures until end of turn.
**Scryfall type line**: Instant
**Status**: PASS

- Name: correct ("Spare from Evil")
- Cost: {1}{W} -- correct
- Type: Instant -- correct
- Oracle text: matches
- Implementation uses `UntilEndOfTurnProtection` with a `CreatureFilter::Not(HasSubtype("Human"))` filter, which correctly models "protection from non-Human creatures"
- Correctly collects all creatures controlled by the caster on the battlefield and grants them the protection
- Spell moves to graveyard after resolve via `move_spell_after_resolve`
- Tests exist in `tier12_cards.rs`
- No issues found
