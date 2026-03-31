# Audit: Mirror-Mad Phantasm

## Official Oracle
- **Name:** Mirror-Mad Phantasm
- **Cost:** {3}{U}{U}
- **Type:** Creature — Spirit
- **Oracle:** Flying. {1}{U}: Mirror-Mad Phantasm's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
- **P/T:** 5/1

## Implementation: `mtg-engine/src/cards/mirror_mad_phantasm.rs`
- **Name:** Mirror-Mad Phantasm -- CORRECT
- **Cost:** {3}{U}{U} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Spirit -- CORRECT
- **P/T:** 5/1 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Activated ability:** {1}{U}, no tap required -- CORRECT
- **on_activate_ability:** Shuffles into library, reveals until Mirror-Mad Phantasm found -- CORRECT

## Notes
- Simplified shuffle: card is appended to bottom of library rather than shuffled to a random position. Minor simplification.
- The ability correctly handles the case where the entire library is milled without finding a copy.

## Verdict
**PASS** -- No issues found. Shuffle simplification is minor.
