## Audit — 2026-04-01

**Scryfall Oracle text**: If you would draw a card while your library has no cards in it, you win the game instead.
**Scryfall type line**: Creature — Human Wizard
**Status**: PASS

- Name: Laboratory Maniac -- correct
- Cost: {2}{U} -- correct
- Type: Creature -- correct
- Subtypes: Human, Wizard -- correct
- P/T: 2/2 -- correct
- Replacement effect: win instead of drawing from empty library -- noted in oracle_text. The actual replacement effect logic would need to be in the engine's draw code (not in the card file itself). The card data correctly describes the ability.
- Tests exist in tier14_cards.rs (tests win condition and controller-only restriction)

No issues found with the card data. The replacement effect is a special rule that must be handled by the engine draw logic.
