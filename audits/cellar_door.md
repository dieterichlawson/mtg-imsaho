# Audit: Cellar Door

## Oracle Text (Scryfall)
> {3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.

## Card Data
- **Name**: Cellar Door -- correct
- **Mana Cost**: {2} -- correct
- **Type**: Artifact -- correct
- **Subtypes**: none -- correct

## Activated Ability
- **Cost**: {3}, {T} -- correct
- **Target**: TargetRequirement::PlayerOnly -- correct (oracle says "Target player")
- **Bottom-of-library mill**: correct. Code uses `library_order[last_idx]` (line 67-68), and `library_order[0]` is top of library, so last index is bottom.
- **Creature check**: correct. Checks `CardType::Creature` on the milled card via registry lookup.
- **Token creation**: correct. Creates a 2/2 black Zombie creature token with subtype "Zombie", owned by `controller` (the controller of Cellar Door, matching oracle "you create").

## Issues

### Issue 1: Inaccurate oracle_text field (cosmetic)
**Severity**: Low (cosmetic, does not affect gameplay)

The `oracle_text` field in `card_data()` reads:
> `"{3}, {T}: Target player mills the bottom card of their library. If a creature card is milled this way, create a 2/2 black Zombie creature token."`

The actual Scryfall oracle text is:
> `"{3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token."`

Differences:
1. "mills the bottom card" vs "puts the bottom card of their library into their graveyard" -- the card predates the mill keyword and Scryfall's oracle text does not use "mill."
2. "If a creature card is milled this way, create" vs "If it's a creature card, you create" -- minor wording difference; "you" is significant for clarity on who creates the token, though mechanically implemented correctly.

### Issue 2: Test does not distinctly verify bottom-of-library behavior
**Severity**: Low (test quality)

In `mtg-engine/tests/tier15_cards.rs` line 469, the test inserts the creature card at index 0 (top of library):
```rust
state.get_player_mut(P1).library_order.insert(0, card);
```
Since this is the only card in the library, top == bottom, so it does not verify that the implementation specifically mills from the bottom rather than the top. A stronger test would place multiple cards and verify only the bottom one is milled.

## Verdict
**PASS** -- The gameplay logic is correct. The only issues are a cosmetic oracle text mismatch and a test that could be more thorough.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
**Type line**: Artifact
**Status**: ISSUE

### Code issues
1. **Oracle text mismatch**: Oracle says "Target player puts the bottom card of their library into their graveyard" but code oracle_text says "Target player mills the bottom card of their library." The oracle uses the older "puts into graveyard" template, not the "mills" keyword. Code oracle_text should be updated. No gameplay impact — behavior correctly moves the bottom card to graveyard.
