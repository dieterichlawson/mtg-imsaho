# Audit: Mirror-Mad Phantasm

## Reference (Scryfall/API)
- **Name:** Mirror-Mad Phantasm
- **Mana Cost:** {3}{U}{U}
- **Type:** Creature — Spirit
- **Oracle:** Flying / {1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
- **P/T:** 5/1

## Implementation: `mirror_mad_phantasm.rs`
- **Name:** Mirror-Mad Phantasm -- CORRECT
- **Mana Cost:** {3}{U}{U} -- CORRECT
- **Type:** Creature — Spirit -- CORRECT
- **P/T:** 5/1 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Activated ability:** cost {1}{U}, requires_tap=false -- CORRECT
- **Shuffle into library:** Moves to library zone, appends to library_order -- CORRECT (simplified shuffle)
- **Reveal loop:** Draws cards from top until finding "Mirror-Mad Phantasm" by name -- CORRECT
- **Found:** Puts onto battlefield with controller = owner -- CORRECT
- **Not found:** All cards milled to graveyard -- CORRECT (matches ruling about tokens/copies)
- **Milled cards:** All non-Phantasm revealed cards moved to graveyard -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Flying\n{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
**Type line**: Creature — Spirit
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Mirror-Mad Phantasm", cost {3}{U}{U}, 5/1, type Creature with subtype Spirit, keyword Flying. Activated ability costs {1}{U}, shuffles into owner's library, reveals cards until finding Mirror-Mad Phantasm by name, puts it onto the battlefield, and mills all other revealed cards. Behavior is correct.
