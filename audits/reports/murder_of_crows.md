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

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Murder of Crows  
**Type**: Creature — Bird | **Cost**: {3}{U}{U} | **P/T**: 4/4  
**Oracle text**: "Flying\nWhenever another creature dies, you may draw a card. If you do, discard a card."

### Checks
- Name: "Murder of Crows" -- PASS
- Cost: {3}{U}{U} -- PASS
- Type: Creature -- PASS
- Subtypes: Bird -- PASS
- P/T: 4/4 -- PASS
- Keywords: Flying -- PASS
- Trigger: AnyCreatureDies (another creature) -- PASS
- Behavior (may draw): Presents YesNo choice to controller -- PASS
- Behavior (discard): On yes, draws 1 card then presents discard choice (or auto-discards if only 1 card in hand) -- PASS
- Zone check: Only triggers while on battlefield -- PASS

**Verdict: PASS**
