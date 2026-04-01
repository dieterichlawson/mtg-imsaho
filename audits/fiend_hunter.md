## Audit — 2026-04-01

**Scryfall Oracle text**: When Fiend Hunter enters the battlefield, you may exile another target creature.\nWhen Fiend Hunter leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Scryfall type line**: Creature — Human Cleric
**Status**: PASS

- Mana cost {1}{W}{W}: correct.
- Type Creature, subtypes Human Cleric: correct.
- Power/Toughness 1/3: correct.
- ETB trigger: "you may exile another target creature" — correctly uses optional choice presentation.
- Can target own creatures (not just opponent's): correct per Oracle.
- Leaves battlefield trigger returns exiled card: correct.
- Stores exiled creature ID in `card_state["exiled_creature"]`: correct.
- Checks that the card is still in exile before returning: correct.
- TriggerKind::EntersBattlefield and TriggerKind::LeavesBattlefield: correct.
- Tests exist in `card_fixes.rs`, `tier3_cards.rs`, and `card_mechanics.rs`.
