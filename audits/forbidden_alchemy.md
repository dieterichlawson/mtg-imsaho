# Audit: Forbidden Alchemy

## Reference (Scryfall)
- **Name:** Forbidden Alchemy
- **Cost:** {2}{U}
- **Type:** Instant
- **Oracle:** Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard. Flashback {6}{B}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{U})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Flashback cost: CORRECT ({6}{B})
- Looks at top 4 cards: CORRECT (drains 4 from library_order)
- Player chooses one for hand: CORRECT (ChooseFromRevealed choice)
- Rest go to graveyard: CORRECT
- P/T: CORRECT (N/A)

## Issues
None found.
