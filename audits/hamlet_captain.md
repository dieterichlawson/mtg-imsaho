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

## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever Hamlet Captain attacks or blocks, other Humans you control get +1/+1 until end of turn.
**Scryfall type line**: Creature — Human Warrior
**Status**: ISSUE

1. **Oracle says "other Humans" not "other Human creatures"**: The current Oracle text says "other Humans you control" which includes any permanent with the Human subtype. The code filters for creatures (o.power.is_some()) and checks Human subtype — this is fine in practice since Humans are always creatures, but strictly speaking the Oracle text says "Humans" not "Human creatures."
2. **Minor: triggered_abilities lists both Attacks and Blocks**: This is correct and matches the Oracle text. No issue.

Note: The implementation correctly checks both obj.subtypes and registry subtypes for Human (line 67-70), which is good for catching token Humans. Tests exist (tier12_cards.rs).
