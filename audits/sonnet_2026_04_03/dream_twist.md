## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target player mills three cards.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues
- Oracle text field incomplete in mtg-engine/src/cards/isd/dream_twist.rs:22
  - Oracle text says: `Target player mills three cards.\nFlashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`
  - Code does: Stores only `"Target player mills three cards."` in oracle_text field, omitting flashback text

### Tricky interactions checked
- "Target player" allows any player including self: PASS (TargetRequirement::PlayerOnly correct)
- Flashback spells get exiled after resolution: PASS (cast_with_flashback flag properly set, move_spell_after_resolve checks flag)
- Mill handles empty library correctly: PASS (mill_cards function breaks when library_order.is_empty())
- Flashback timing restrictions respected: PASS (instant can be cast at any time)
- Mana value vs flashback cost for counterspells: PASS (mana value remains 1 regardless of flashback cost)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic milling 3 cards: `mtg-engine/tests/flashback.rs:229`
- Flashback functionality from graveyard: NOT TESTED
- Self-targeting allowed: `mtg-engine/tests/witchbane_orb.rs:70`
- Exile after flashback resolution: NOT TESTED
- Mill with insufficient library cards: NOT TESTED