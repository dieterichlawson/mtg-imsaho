## Audit — 2026-04-01

**Scryfall Oracle text**: As Nevermore enters the battlefield, choose a nonland card name.\nSpells with the chosen name can't be cast.
**Scryfall type line**: Enchantment
**Status**: ISSUE

- Name: Correct ("Nevermore")
- Cost: {1}{W}{W} - Correct
- Type: Enchantment - Correct
- Oracle text matches.

Issues:
1. **Auto-selection of card name instead of player choice**: The implementation automatically picks the first nonland card from the opponent's hand. The Oracle text says the controller should choose any nonland card name (not limited to cards in opponent's hand -- you can name any legal card name). This is a simplification that significantly alters card behavior.
2. **Default name fallback**: If no nonland card is found in opponent's hand, it defaults to "Lightning Bolt" which is arbitrary and incorrect behavior.
3. **Looks at opponent's hand**: The card should let the player choose any nonland card name; it should NOT look at the opponent's hand. Nevermore is a blind naming effect, not a hand-disruption card.

- Tests: tier14_cards.rs has `nevermore_prevents_named_spell` and `nevermore_allows_other_spells`. Tests manually set the named card, so they don't exercise the auto-selection bug.
