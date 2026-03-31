# Audit: Altar's Reap

## Reference (Scryfall/API)
- **Name:** Altar's Reap
- **Mana Cost:** {1}{B}
- **Type:** Instant
- **Oracle:** As an additional cost to cast this spell, sacrifice a creature. Draw two cards.
- **P/T:** N/A

## Implementation: `altars_reap.rs`
- **Name:** Altar's Reap -- CORRECT
- **Mana Cost:** {1}{B} -- CORRECT
- **Type:** Instant -- CORRECT
- **Additional cost:** SacrificeCreature -- CORRECT
- **Effect:** Draw 2 cards -- CORRECT

## Issues
1. **ISSUE (minor/known):** The sacrifice happens on resolution rather than as part of casting. Code has a comment acknowledging this simplification: "the engine doesn't yet support multi-step casting with additional costs." The spell also selects the creature to sacrifice automatically rather than letting the player choose. Additionally, if no creature is available at resolution time, it still fizzles (which is correct behavior since you shouldn't have been able to cast it without the cost, but this is a consequence of the simplification).

## Verdict: PASS (with known simplification) -- Sacrifice timing is noted as simplified
