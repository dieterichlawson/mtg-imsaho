## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
**Type line**: Creature — Insect
**Status**: ISSUE

### Code issues

- Incomplete discard implementation in `mtg-engine/src/cards/isd/brain_weevil.rs` lines 65-75
  - Oracle text says: `Target player discards two cards`
  - Code does: When target player has >2 cards, only prompts for one discard ("choose a card to discard (1 of 2)") and has no `on_discard_choice` method to handle the second discard. After the first card is chosen, the engine calls the non-existent `on_discard_choice` hook, finds nothing, and the ability ends with only one card discarded instead of two.

### Tricky interactions checked

- **Mandatory discard**: PASS — No "may" in oracle text, correctly implemented as mandatory
- **Target selection**: PASS — `TargetRequirement::PlayerOnly` allows targeting any player including self
- **Sacrifice as cost**: PASS — `SacrificeCost::SacrificeThis` correctly implements sacrifice as a cost (happens before effect can be responded to)
- **Sorcery speed timing**: PASS — `sorcery_speed_only: true` correctly restricts activation
- **Auto-discard for ≤2 cards**: PASS — Lines 54-64 correctly discard all cards when player has 2 or fewer cards
- **Multiple discard handling**: FAIL — Lines 65-75 only handle first of two discards when player has >2 cards

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- **Auto-discard when player has ≤2 cards**: `mtg-engine/tests/tier8_cards.rs:93` (`brain_weevil_forces_discard`)
- **Multiple discard when player has >2 cards**: NOT TESTED
- **Intimidate keyword**: `mtg-engine/tests/tier8_cards.rs:129` (`brain_weevil_has_intimidate`)
- **Sacrifice as cost timing**: NOT TESTED
- **Sorcery speed restriction**: NOT TESTED
- **Targeting any player**: NOT TESTED