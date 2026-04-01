## Audit — 2026-04-01

**Scryfall Oracle text**: This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
**Scryfall type line**: Sorcery
**Status**: PASS

- Mana cost {8}{R}: correct
- Card type Sorcery: correct
- Cost reduction via modified_cost: correct
- Reduction capped at 8 (can't go below {R}): correct
- Deals 13 damage to each creature on battlefield: correct
- Uses NonCombatDamageDealt event: correct
- Uses move_spell_after_resolve: correct
- Tests exist in tier12_cards.rs covering damage, cost reduction, and castability

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: This spell costs {1} less to cast for each creature on the battlefield. Blasphemous Act deals 13 damage to each creature.
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. Cost reduction, minimum cost ({R}), damage to all creatures, NonCombatDamageDealt events all correct.
