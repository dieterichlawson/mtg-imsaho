## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Deathtouch
When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
**Type line**: Creature — Human Shaman
**Status**: ISSUE

### Code issues
- Missing enchantment legality check when attaching curse to target player (`mtg-engine/src/engine.rs:2598-2614` and `mtg-engine/src/cards/isd/bitterheart_witch.rs:13-31`)
  - Oracle text says: `When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.`
  - Ruling states: `The Curse must be legally able to enchant the player. For example, if the player has protection from red, you couldn't put a red Curse onto the battlefield this way.`
  - Code does: Directly sets `attached_to_player = Some(*pid)` without checking if the curse can legally enchant the target player (e.g., player has protection from curse's color). Lines 14-16 include ALL players as valid targets without filtering for enchantment legality.

### Tricky interactions checked
- "you may" optionality: PASS — YesNo choice correctly presented, player can decline
- Target player selection: PASS — Any player can be targeted (including self), confirmed by tests
- Curse subtype filtering: PASS — Correctly searches library for cards with "Curse" subtype
- Multiple curses handling: PASS — Presents choice when multiple curses found
- Library shuffling: PASS — Shuffles after attachment or when declining/no curses found
- Enchantment legality validation: FAIL — Missing protection/legality checks per ruling

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic curse finding and attachment: `mtg-engine/tests/tier15_cards.rs:175-214`  
- Attaching curse to self: `mtg-engine/tests/tier15_cards.rs:216-249`
- Declining to search: `mtg-engine/tests/tier15_cards.rs:251-279`
- "you may" choice: `mtg-engine/tests/tier15_cards.rs:191` (YesNo awaiting_action)
- Target player choice: `mtg-engine/tests/tier15_cards.rs:200-208`
- Enchantment legality with protection: NOT TESTED