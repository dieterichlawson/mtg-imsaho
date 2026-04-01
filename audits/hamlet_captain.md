## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever Hamlet Captain attacks or blocks, other Human creatures you control get +1/+1 until end of turn.
**Scryfall type line**: Creature — Human Warrior
**Status**: PASS

- Mana cost {1}{G}: correct
- 2/2 stats: correct
- Subtypes Human Warrior: correct
- Triggers on both attacks and blocks: correct (two TriggeredAbilityDefs, on_attacks and on_blocks both call buff_humans)
- Buffs other Human creatures you control (excluding self) with +1/+1 until end of turn: correct
- Checks subtypes on both object and card_data for Human: correct
- Tests exist in tier12_cards.rs covering attack and block buffs
