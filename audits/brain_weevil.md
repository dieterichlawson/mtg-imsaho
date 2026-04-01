## Audit — 2026-04-01

**Scryfall Oracle text**: Intimidate
Sacrifice Brain Weevil: Target player discards two cards. Activate only as a sorcery.
**Scryfall type line**: Creature — Insect
**Status**: PASS

- Mana cost {3}{B}: correct
- 1/1 stats: correct
- Subtype Insect: correct
- Keyword Intimidate: correct
- Activated ability: sacrifice self, target player discards two, sorcery speed: correct
- sacrifice_cost: SacrificeCost::SacrificeThis: correct
- target_requirement: PlayerOnly: correct
- sorcery_speed_only: true: correct
- Discard implementation handles edge case of 2 or fewer cards in hand: correct
- For 3+ cards, presents choice to target player: correct
- Tests exist in tier8_cards.rs covering discard and intimidate

## Audit — 2026-04-01 (independent re-audit)

**Scryfall Oracle text**: Intimidate. Sacrifice Brain Weevil: Target player discards two cards. Activate only as a sorcery.
**Scryfall type line**: Creature — Insect
**Status**: PASS

No issues found. Sacrifice cost, sorcery speed, target player choice, and discard handling all correct.
