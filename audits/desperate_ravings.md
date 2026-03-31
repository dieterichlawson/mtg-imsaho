# Audit: Desperate Ravings

## Reference (Scryfall)
- **Name:** Desperate Ravings
- **Cost:** {1}{R}
- **Type:** Instant
- **Oracle:** Draw two cards, then discard a card at random. Flashback {2}{U}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{R})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Flashback cost: CORRECT ({2}{U})
- P/T: CORRECT (N/A)
- on_resolve draws 2 cards: CORRECT
- on_resolve discards at random: CORRECT (uses `choose(&mut rand::thread_rng())`)

## Issues
None found.
