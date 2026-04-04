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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Defender\n{T}: Exchange your life total with this creature's toughness.
**Type line**: Creature — Plant
**Mana Cost**: {3}{G}
**P/T**: 0/13
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Tree of Redemption", cost {3}{G}, type Creature, subtype Plant, P/T 0/13, Defender keyword. Oracle text in code says "Tree of Redemption's toughness" vs oracle "this creature's toughness" — minor self-referential wording difference acceptable for card implementations. Activated ability requires tap, costs no mana, exchanges life total with effective toughness. on_activate_ability correctly reads effective_toughness, swaps life and base toughness values, and logs the exchange. All correct.
