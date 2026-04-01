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

## Audit — 2026-04-01

**Scryfall Oracle text**: When Witchbane Orb enters the battlefield, destroy all Curses attached to you. You have hexproof.
**Scryfall type line**: Artifact
**Mana cost**: {4}
**Status**: ISSUE

1. **Player hexproof not implemented** (`mtg-engine/src/cards/witchbane_orb.rs`): The code comment acknowledges that "player hexproof is not implemented (would need a new engine system)." The ETB curse destruction works, but the continuous "You have hexproof" static ability is not enforced. Opponents can still target the player with spells/abilities.
