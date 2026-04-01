## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
**Scryfall type line**: Instant
**Status**: ISSUE

- Mana cost {1}{B}: correct
- Card type Instant: correct
- Additional cost SacrificeCreature: correct
- Uses crate::destruction::sacrifice: correct
- Uses crate::engine::draw_cards: correct
- Uses move_spell_after_resolve: correct

Issues found:
1. **Sacrifice timing is wrong**: The implementation sacrifices on resolution (line 42-48), but the sacrifice is an additional cost to *cast* the spell. The code comment acknowledges this as a simplification. However, this means if the spell is countered, no creature was sacrificed, which is incorrect — the creature should already be gone. This is a known engine limitation per the comment.
2. **No player choice for sacrifice target**: The code picks the first creature found via `.find()` rather than letting the player choose which creature to sacrifice. The Oracle text doesn't specify "choose" but in MTG the controller chooses which creature to sacrifice as part of paying costs.
3. **move_spell_after_resolve called even when spell fizzles**: If no creature is found, the spell should still go to graveyard, but the comment says "the spell fizzles (no effect)" — in real MTG, if you can't pay the additional cost, you can't cast it at all. Since the engine handles this at resolution, move_spell_after_resolve is called regardless, which is fine for cleanup.

Test exists in tier8_cards.rs.

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: As an additional cost to cast this spell, sacrifice a creature. Draw two cards.
**Scryfall type line**: Instant
**Status**: ISSUE

1. **Auto-selects sacrifice target** (altars_reap.rs:42-44): Uses `.find()` instead of presenting player choice. Player should choose which creature to sacrifice.
2. **Sacrifice timing simplified**: Sacrifice happens on resolution instead of as a casting cost. Known engine limitation per code comments.
