## Audit — 2026-04-01

**Scryfall Oracle text**: (Front — Garruk Relentless) When Garruk Relentless has two or fewer loyalty counters on him, transform him.
0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him.
0: Create a 2/2 green Wolf creature token.
(Back — Garruk, the Veil-Cursed) +1: Create a 1/1 black Wolf creature token with deathtouch.
-1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.
-3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Scryfall type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Status**: ISSUE

- Mana cost {3}{G}: correct
- Starting loyalty 3: correct
- Supertype Legendary: correct
- Subtype Garruk: correct
- Front face 0-ability (fight creature): correctly deals 3 damage to creature, creature deals its power back to Garruk as loyalty removal
- Front face 0-ability (Wolf token): correctly creates 2/2 green Wolf creature token
- Transform condition (2 or fewer loyalty): correctly checks after each ability activation and sets is_transformed
- ISSUE: Back face (Garruk, the Veil-Cursed) abilities are not implemented. The back face has three loyalty abilities (+1 deathtouch Wolf, -1 sacrifice/tutor, -3 pump) which are completely missing. The implementation just renames to "Garruk, the Veil-Cursed" but provides no usable abilities after transform.
- ISSUE: The transform trigger should be a state-based check, not only checked after loyalty ability activation. Oracle says "When Garruk Relentless has two or fewer loyalty counters on him, transform him" — this is a triggered ability that should fire whenever the condition becomes true (e.g., from damage).
- ISSUE: The 0-ability target selection auto-picks the strongest opponent creature rather than allowing player choice.
- Tests exist in tier15_cards.rs covering Wolf token creation and transform at 2 loyalty.
