# Audit: Murder of Crows

## Official Oracle
- **Name:** Murder of Crows
- **Cost:** {3}{U}{U}
- **Type:** Creature — Bird
- **Oracle:** Flying. Whenever another creature dies, you may draw a card. If you do, discard a card.
- **P/T:** 4/4

## Implementation: `mtg-engine/src/cards/murder_of_crows.rs`
- **Name:** Murder of Crows -- CORRECT
- **Cost:** {3}{U}{U} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Bird -- CORRECT
- **P/T:** 4/4 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Triggered ability:** AnyCreatureDies -- CORRECT
- **on_any_creature_dies:** Presents yes/no choice to draw then discard -- CORRECT

## Verdict
**PASS** -- No issues found.
