# Audit: Avacyn's Pilgrim

## Oracle Text (Scryfall)
- **Name:** Avacyn's Pilgrim
- **Mana Cost:** {G}
- **Type:** Creature — Human Monk
- **P/T:** 1/1
- **Oracle Text:** {T}: Add {W}.

## Implementation File
`mtg-engine/src/cards/isd/avacyns_pilgrim.rs`

## Card Data Checks
- **Name:** Correct ("Avacyn's Pilgrim")
- **Mana Cost:** Correct ({G})
- **Card Types:** Correct (Creature)
- **Subtypes:** Correct (Human, Monk)
- **P/T:** Correct (1/1)
- **Oracle Text:** Correct

## Behavior Checks
- **Mana ability:** Produces {W} via tap, correct. Checks battlefield zone, untapped, and not summoning sick -- all correct.

## Verdict: PASS
