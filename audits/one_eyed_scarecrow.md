# Audit: One-Eyed Scarecrow

## Reference (Scryfall/API)
- **Name:** One-Eyed Scarecrow
- **Mana Cost:** {3}
- **Type:** Artifact Creature -- Scarecrow
- **Oracle:** Defender / Creatures with flying your opponents control get -1/-0.
- **P/T:** 2/3

## Implementation: `one_eyed_scarecrow.rs`
- **Name:** One-Eyed Scarecrow -- CORRECT
- **Mana Cost:** {3} -- CORRECT
- **Type:** Artifact Creature -- Scarecrow -- CORRECT (card_types: [Artifact, Creature], subtypes: ["Scarecrow"])
- **P/T:** 2/3 -- CORRECT
- **Keywords:** Defender -- CORRECT
- **Continuous effect:** ModifyPT { power: -1, toughness: 0, scope: Global(And([Opponents, HasKeyword(Flying)])) } -- CORRECT

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Defender\nCreatures with flying your opponents control get -1/-0.
**Type line**: Artifact Creature -- Scarecrow
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {3}, 2/3, Artifact Creature -- Scarecrow, Defender keyword, continuous effect applies -1/-0 to opponents' creatures with flying. Behavior is correct.
