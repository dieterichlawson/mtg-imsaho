# Audit: Maw of the Mire

## Official Oracle
- **Name:** Maw of the Mire
- **Cost:** {4}{B}
- **Type:** Sorcery
- **Oracle:** Destroy target land. You gain 4 life.

## Implementation: `mtg-engine/src/cards/maw_of_the_mire.rs`
- **Name:** Maw of the Mire -- CORRECT
- **Cost:** {4}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Target:** Land (PermanentWithFilter HasCardType Land) -- CORRECT
- **on_resolve:** Destroys target land via try_destroy, gains 4 life with LifeChanged event -- CORRECT

## Verdict
**PASS** -- No issues found.
