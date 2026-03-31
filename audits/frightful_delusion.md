# Audit: Frightful Delusion

## Reference (Scryfall)
- **Name:** Frightful Delusion
- **Cost:** {2}{U}
- **Type:** Instant
- **Oracle:** Counter target spell unless its controller pays {1}. That player discards a card.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{U})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Target requirement: CORRECT (Spell)
- Counter unless pays {1}: CORRECT (checks mana_pool.total() >= 1, offers PayOrNot choice)
- That player discards a card: CORRECT (forces discard after counter or if can't pay)
- P/T: CORRECT (N/A)

## Issues
None found. (The PayOrNot resolution handler in engine.rs correctly forces a discard regardless of whether the player pays or not.)
