## Audit — 2026-04-01

**Scryfall Oracle text**: When Slayer of the Wicked enters the battlefield, you may destroy target Vampire, Werewolf, or Zombie.
**Scryfall type line**: Creature — Human Soldier
**Mana cost**: {3}{W}
**P/T**: 3/2
**Status**: PASS

Implementation correctly models:
- Name, mana cost {3}{W}, type Creature, subtypes Human/Soldier, P/T 3/2
- ETB triggered ability: "you may destroy target Vampire, Werewolf, or Zombie"
- Uses optional target choice (correctly models "you may")
- Filters targets by Vampire, Werewolf, or Zombie subtypes
- Tests: `slayer_of_the_wicked_destroys_zombie` in tier3_cards.rs

No issues found.

## Audit — 2026-04-01

**Scryfall Oracle text**: When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
**Scryfall type line**: Creature — Human Soldier
**Status**: ISSUE

1. **Subtype check misses tokens** (slayer_of_the_wicked.rs:41-43): The target filter only checks `registry.card_data(o.card_id)` for subtypes. Tokens (card_id=0) with Vampire/Werewolf/Zombie subtypes will not be found as valid targets. Should also check `o.subtypes`.
