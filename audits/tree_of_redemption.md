# Audit: Tree of Redemption

## Scryfall Reference
- **Name:** Tree of Redemption
- **Cost:** {3}{G}
- **Type:** Creature — Plant
- **Oracle:** Defender / {T}: Exchange your life total with this creature's toughness.
- **P/T:** 0/13

## Implementation: `mtg-engine/src/cards/tree_of_redemption.rs`
- Name: "Tree of Redemption" -- MATCH
- Cost: {3}{G} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Plant"] -- MATCH
- P/T: 0/13 -- MATCH
- Keywords: [Defender] -- MATCH
- Activated ability: {T} (requires_tap: true, free mana cost) -- MATCH
- Behavior: Gets effective toughness, exchanges with life total, sets obj.toughness to old life -- MATCH
- Emits LifeChanged event -- CORRECT

## Verdict
**PASS** — Correctly implemented including the life exchange mechanic.
