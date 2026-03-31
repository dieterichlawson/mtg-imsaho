# Audit: Manor Gargoyle

## Oracle (Official)
- **Name:** Manor Gargoyle
- **Cost:** {5}
- **Type:** Artifact Creature — Gargoyle
- **Oracle:** Defender. Manor Gargoyle is indestructible as long as it has defender. {1}: Until end of turn, Manor Gargoyle loses defender and gains flying.
- **P/T:** 4/4

## Implementation
- Name: "Manor Gargoyle" -- CORRECT
- Cost: {5} -- CORRECT
- Types: [Artifact, Creature] -- CORRECT
- Subtypes: ["Gargoyle"] -- CORRECT
- P/T: 4/4 -- CORRECT
- Keywords: [Defender] -- CORRECT
- Oracle text matches -- CORRECT
- Conditional indestructible when it has defender via ConditionalKeyword -- CORRECT
- Activated ability {1}: loses defender, gains flying until end of turn -- CORRECT
- Uses until_end_of_turn_removed_keywords for removing defender -- CORRECT
- Uses UntilEndOfTurnKeyword for granting flying -- CORRECT

## Issues
None.

## Verdict: PASS
