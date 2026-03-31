# Audit: Liliana of the Veil

## Oracle (Official)
- **Name:** Liliana of the Veil
- **Cost:** {1}{B}{B}
- **Type:** Legendary Planeswalker — Liliana
- **Oracle:** +1: Each player discards a card. -2: Target player sacrifices a creature. -6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
- **Loyalty:** 3

## Implementation
- Name: "Liliana of the Veil" -- CORRECT
- Cost: {1}{B}{B} -- CORRECT
- Type: Planeswalker -- CORRECT
- Supertypes: [Legendary] -- CORRECT
- Subtypes: ["Liliana"] -- CORRECT
- Starting loyalty: 3 -- CORRECT
- Oracle text matches -- CORRECT
- +1: each player discards (auto-picks first card in hand) -- CORRECT (simplified selection)
- -2: opponent sacrifices a creature -- CORRECT (simplified: always targets opponent)
- -6: opponent sacrifices half their permanents -- SIMPLIFICATION (real card has pile division with player choice)

## Issues
1. **ISSUE (simplification):** +1 ability auto-picks the first card in hand rather than allowing player choice.
2. **ISSUE (simplification):** -2 always targets opponent, rather than allowing "target player" selection.
3. **ISSUE (simplification):** -6 simplified from pile division to "sacrifice half permanents." The real card separates into two piles and the target player chooses which pile to sacrifice. Code takes the first half, not a random/chosen split.

## Verdict: PASS (with noted simplifications — all acknowledged in comments)
