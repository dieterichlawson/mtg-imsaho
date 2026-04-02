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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: {1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
**Type line**: Artifact
**Mana Cost**: {1}
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch**: Oracle says `"Sacrifice this artifact"` but code has `"Sacrifice Traveler's Amulet"`. Modern oracle template uses "this artifact" as self-reference.
### Behavior
Correct. Activated ability costs {1} with SacrificeCost::SacrificeThis. on_activate_ability searches library for a basic land (checks CardType::Land and Supertype::Basic), moves it to hand. Shuffle is noted as no-op. All mechanical behavior matches oracle.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Sacrifice this artifact" (was "Sacrifice Traveler's Amulet"). Doc comment updated. Behavior unchanged.
