## Audit — 2026-04-01

**Scryfall Oracle text**: Target opponent loses 3 life.
Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Scryfall type line**: Sorcery
**Status**: PASS

- Mana cost {B}: correct
- Card type Sorcery: correct
- Flashback {5}{R}: correct
- Target requirement PlayerOnly: correct
- is_valid_target excludes caster (opponent only): correct
- Loses 3 life (not damage): correct — implemented as direct life subtraction, not damage
- Uses move_spell_after_resolve: correct
- Emits LifeChanged event: correct
- Tests exist in tier2_spells.rs and flashback.rs

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Target opponent loses 3 life. Flashback {5}{R}
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. Life loss (not damage) correctly implemented. Opponent-only targeting correct. Flashback cost correct.
