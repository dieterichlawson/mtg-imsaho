## Audit — 2026-04-01

**Scryfall Oracle text**: Flying
As long as you control a Human, Angelic Overseer has hexproof and is indestructible.
**Scryfall type line**: Creature — Angel
**Status**: PASS

- Mana cost {3}{W}{W}: correct
- 5/3 stats: correct
- Subtype Angel: correct
- Keyword Flying: correct
- Conditional Hexproof with YouControlSubtype("Human"): correct
- Conditional Indestructible with YouControlSubtype("Human"): correct
- Both use EffectScope::OnSelf: correct
- Tests exist in tier12_cards.rs covering flying, hexproof/indestructible with human, and surviving destroy

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Flying. As long as you control a Human, this creature has hexproof and indestructible.
**Scryfall type line**: Creature — Angel
**Status**: PASS

No issues found. Conditional keywords implemented correctly with continuous effects.
