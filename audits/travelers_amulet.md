# Audit: Traveler's Amulet

## Scryfall Reference
- **Name:** Traveler's Amulet
- **Cost:** {1}
- **Type:** Artifact
- **Oracle:** {1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/travelers_amulet.rs`
- Name: "Traveler's Amulet" -- MATCH
- Cost: {1} -- MATCH
- Types: Artifact -- MATCH
- Activated ability: {1}, sacrifice this -- MATCH
- Behavior: Searches library for basic land (CardType::Land + Supertype::Basic), puts into hand -- MATCH
- Shuffle noted as no-op (engine limitation) -- OK

## Verdict
**PASS** — Correctly implemented.
