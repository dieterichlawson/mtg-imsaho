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
