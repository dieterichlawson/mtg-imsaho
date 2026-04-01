## Audit — 2026-04-01

**Scryfall Oracle text**: When Ghoulraiser enters the battlefield, return a Zombie creature card at random from your graveyard to your hand.
**Scryfall type line**: Creature — Zombie
**Status**: PASS

- Mana cost {1}{B}{B}: correct
- 2/2 stats: correct
- Subtype Zombie: correct
- ETB trigger: searches graveyard for Zombie creature cards, picks one at random, returns to hand: correct
- Random selection uses shuffle + pick first: correct
- Correctly checks both creature type and Zombie subtype: correct
- Tests exist in tier11_cards.rs covering ETB return

## Audit — 2026-04-01

**Scryfall Oracle text**: When this creature enters, return a Zombie card at random from your graveyard to your hand.
**Scryfall type line**: Creature — Zombie
**Status**: ISSUE

1. **Oracle says "Zombie card" not "Zombie creature card"**: The code filters for cards that are both creatures AND Zombies (line 53-57). The Oracle text says "a Zombie card" meaning any card with subtype Zombie, not just creature cards. In practice this rarely matters since Zombie subtype is almost exclusively on creatures, but it's technically incorrect.
