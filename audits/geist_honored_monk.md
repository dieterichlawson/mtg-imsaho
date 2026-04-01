## Audit — 2026-04-01

**Scryfall Oracle text**: Vigilance
Geist-Honored Monk's power and toughness are each equal to the number of creatures you control.
When Geist-Honored Monk enters the battlefield, create two 1/1 white Spirit creature tokens with flying.
**Scryfall type line**: Creature — Human Monk
**Status**: PASS

- Mana cost {3}{W}{W}: correct
- Base P/T */* (stored as 0/0): correct
- Subtypes Human Monk: correct
- Keyword Vigilance: correct
- dynamic_pt counts creatures controller controls on battlefield: correct
- ETB trigger creates two 1/1 white Spirit tokens with flying: correct
- Tests exist in tier5_cards.rs covering dynamic P/T and token creation
