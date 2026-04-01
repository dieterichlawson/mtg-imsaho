## Audit — 2026-04-01

**Scryfall Oracle text**: Return two cards at random from your graveyard to your hand.
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: Make a Wish -- correct
- Cost: {3}{G} -- correct
- Type: Sorcery -- correct
- Effect: return two random graveyard cards to hand -- correctly implemented
- Excludes tokens from graveyard selection -- correct (tokens cease to exist in graveyard per rules, but filtering them is good practice)
- Excludes itself from selection -- correct
- Uses random shuffling -- correct
- Tests exist in innistrad_simple_cards.rs

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Return two cards at random from your graveyard to your hand.
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. Scryfall ruling: "If you only have one card in your graveyard when Make a Wish resolves, that card will be returned to your hand." The implementation uses .take(2) which handles 0, 1, or 2+ cards correctly.
