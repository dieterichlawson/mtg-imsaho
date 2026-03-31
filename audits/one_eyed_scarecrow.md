# Audit: One-Eyed Scarecrow

## Official Oracle
- **Name:** One-Eyed Scarecrow
- **Cost:** {3}
- **Type:** Artifact Creature — Scarecrow
- **Oracle:** Defender. Creatures with flying your opponents control get -1/-0.
- **P/T:** 2/3

## Implementation: `mtg-engine/src/cards/one_eyed_scarecrow.rs`
- **Name:** One-Eyed Scarecrow -- CORRECT
- **Cost:** {3} -- CORRECT
- **Type:** Artifact, Creature -- CORRECT
- **Subtypes:** Scarecrow -- CORRECT
- **P/T:** 2/3 -- CORRECT
- **Keywords:** Defender -- CORRECT
- **Continuous effect:** ModifyPT power:-1 toughness:0, Global(Opponents AND HasKeyword(Flying)) -- CORRECT

## Verdict
**PASS** -- No issues found.
