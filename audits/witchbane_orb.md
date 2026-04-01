## Audit — 2026-04-01

**Scryfall Oracle text**: When Witchbane Orb enters the battlefield, destroy all Curses attached to you.\nYou have hexproof.
**Scryfall type line**: Artifact
**Scryfall mana cost**: {4}
**Status**: ISSUE

Findings:
- Name: Correct.
- Mana cost: {4} — correct.
- Type: Artifact — correct.
- Oracle text: Matches.
- ETB curse destruction: Implemented — finds all permanents with "Curse" subtype attached to the controller and destroys them. Correct.
- **ISSUE: "You have hexproof" is not implemented.** The code comment acknowledges this. While on the battlefield, the controller should have hexproof (cannot be targeted by spells or abilities opponents control). This is a significant missing ability — it is the card's primary ongoing effect.
- Tests: `witchbane_orb_card_data` in innistrad_simple_cards.rs (data-only test, no behavioral test for hexproof).
