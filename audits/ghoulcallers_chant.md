## Audit — 2026-04-01

**Scryfall Oracle text**: Choose one —
* Return target creature card from your graveyard to your hand.
* Return two target Zombie creature cards from your graveyard to your hand.
**Scryfall type line**: Sorcery
**Status**: ISSUE

- Mana cost {B}: correct
- Card type Sorcery: correct
- Oracle text: correct
- ISSUE: The modal choice is not properly enforced. The target_requirement is UpToTargets(2, GraveyardCard) which allows returning 1 or 2 creature cards regardless of type. Oracle requires that if the second mode is chosen (returning 2), both must be Zombie creature cards. The is_valid_target method only checks for creature type, not Zombie subtype, so it allows returning 2 non-Zombie creatures.
- on_resolve correctly returns targeted cards to hand: correct
- Tests exist in tier11_cards.rs covering single creature return and two-zombie return

## Audit — 2026-04-01

**Scryfall Oracle text**: Choose one — / • Return target creature card from your graveyard to your hand. / • Return two target Zombie cards from your graveyard to your hand.
**Scryfall type line**: Sorcery
**Status**: ISSUE

1. **Mode selection not enforced**: The card is a modal spell ("Choose one") but the implementation uses UpToTargets(2, GraveyardCard) without enforcing modal choice. Mode 1 returns exactly 1 creature card (any type). Mode 2 returns exactly 2 Zombie cards. The code allows returning 1 or 2 of any creature card — it doesn't enforce that if 2 targets are chosen, they must be Zombies. The is_valid_target only checks for creature cards, not Zombie subtype. (Line 39-53 in ghoulcallers_chant.rs)
2. **Oracle says "Zombie cards" not "Zombie creature cards"**: Mode 2 targets "Zombie cards" — any card with subtype Zombie, not necessarily creature cards. However, in practice all Zombie cards in Innistrad are creatures so this is very minor.
