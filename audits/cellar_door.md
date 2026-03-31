# Audit: Cellar Door

## Scryfall Reference
- **Name:** Cellar Door
- **Cost:** {2}
- **Type:** Artifact
- **Oracle:** {3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
- **P/T:** N/A
- **Keywords:** none

## Implementation: `cellar_door.rs`
- **Name:** Cellar Door -- CORRECT
- **Cost:** {2} -- CORRECT
- **Type:** Artifact -- CORRECT
- **Subtypes:** none -- CORRECT
- **P/T:** N/A -- CORRECT
- **Keywords:** none -- CORRECT
- **Activated ability:** {3}, {T}, target player -- CORRECT
- **Behavior:** Mills bottom card, creates 2/2 black Zombie if creature -- CORRECT
- **Token subtypes:** ["Zombie"] -- CORRECT

## Issues
None
