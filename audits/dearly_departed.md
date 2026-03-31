# Audit: Dearly Departed

## Scryfall Reference
- **Name:** Dearly Departed
- **Cost:** {4}{W}{W}
- **Type:** Creature -- Spirit
- **Oracle:** Flying. As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
- **P/T:** 5/5
- **Keywords:** Flying

## Implementation: `dearly_departed.rs`
- **Name:** Dearly Departed -- CORRECT
- **Cost:** {4}{W}{W} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Spirit"] -- CORRECT
- **P/T:** 5/5 -- CORRECT
- **Keywords:** [Flying] -- CORRECT
- **Trigger:** AnyCreatureEnters -- CORRECT
- **Behavior:** When in graveyard, Human creatures entering under your control get +1/+1 counter -- CORRECT
- **Zone check:** Checks self is in Graveyard -- CORRECT
- **Human check:** Checks subtypes via registry and object -- CORRECT

## Issues
None
