# Audit: Battleground Geist

## Oracle Text (Scryfall)
- **Name:** Battleground Geist
- **Mana Cost:** {4}{U}
- **Type:** Creature — Spirit
- **P/T:** 3/3
- **Oracle Text:** Flying / Other Spirit creatures you control get +1/+0.

## Implementation File
`mtg-engine/src/cards/isd/battleground_geist.rs`

## Card Data Checks
- **Name:** Correct
- **Mana Cost:** Correct ({4}{U})
- **Card Types:** Correct (Creature)
- **Subtypes:** Correct (Spirit)
- **P/T:** Correct (3/3)
- **Keywords:** Correct (Flying)

## Behavior Checks
- **Continuous effect:** `ModifyPT { power: 1, toughness: 0 }` with scope `GlobalOther(And(You, HasSubtype("Spirit")))` -- correctly gives other Spirit creatures you control +1/+0.

## Verdict: PASS
