## Audit — 2026-04-01

**Scryfall Oracle text**: Defender\n{T}: Exchange your life total with Tree of Redemption's toughness.
**Scryfall type line**: Creature — Plant
**Status**: PASS

- Name: correct ("Tree of Redemption")
- Cost: {3}{G} -- correct
- Type: Creature -- correct
- Subtypes: Plant -- correct
- P/T: 0/13 -- correct
- Keywords: Defender -- correct
- Activated ability: {T} (tap, no mana cost) -- correct
- Exchange logic: reads effective toughness (accounts for counters/buffs), sets life to that value, sets base toughness to old life -- correct
- Uses `effective_toughness` for reading current toughness -- correct
- Modifies `obj.toughness` directly to set new base toughness -- correct approach for this effect
- Tests exist in `tier15_cards.rs`
- No issues found

## Audit — 2026-04-01

**Scryfall Oracle text**: Defender. {T}: Exchange your life total with Tree of Redemption's toughness.
**Scryfall type line**: Creature — Plant
**P/T**: 0/13, **Mana cost**: {3}{G}
**Status**: PASS

No issues found.
