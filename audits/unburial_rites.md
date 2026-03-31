# Audit: Unburial Rites

## Scryfall Reference
- **Name:** Unburial Rites
- **Cost:** {4}{B}
- **Type:** Sorcery
- **Oracle:** Return target creature card from your graveyard to the battlefield. Flashback {3}{W}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/unburial_rites.rs`
- Name: "Unburial Rites" -- MATCH
- Cost: {4}{B} -- MATCH
- Types: Sorcery -- MATCH
- Flashback: {3}{W} -- MATCH
- Behavior: Returns a creature card from graveyard to battlefield -- MATCH
- Handles single vs. multiple choices -- CORRECT
- Uses PendingEffect::ReturnToBattlefield for multi-choice -- CORRECT

## Verdict
**PASS** — Correctly implements reanimation with flashback.
