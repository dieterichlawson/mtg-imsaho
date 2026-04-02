# Audit: Bloodline Keeper // Lord of Lineage

## Oracle Text (Scryfall)

### Front Face
- **Name:** Bloodline Keeper
- **Mana Cost:** {2}{B}{B}
- **Type:** Creature — Vampire
- **P/T:** 3/3
- **Oracle Text:** Flying / {T}: Create a 2/2 black Vampire creature token with flying. / {B}: Transform this creature. Activate only if you control five or more Vampires.

### Back Face
- **Name:** Lord of Lineage
- **Type:** Creature — Vampire
- **P/T:** 5/5
- **Oracle Text:** Flying / Other Vampire creatures you control get +2/+2. / {T}: Create a 2/2 black Vampire creature token with flying.

## Implementation File
`mtg-engine/src/cards/isd/bloodline_keeper.rs`

## Card Data Checks
- **Name:** Correct (front: "Bloodline Keeper", back: "Lord of Lineage")
- **Mana Cost:** Correct ({2}{B}{B})
- **Card Types:** Correct (Creature on both faces)
- **Subtypes:** Correct (Vampire on both faces)
- **P/T:** Correct (front 3/3, back 5/5)
- **Keywords:** Correct (Flying on both faces)
- **Back face continuous effect:** `ModifyPT { power: 2, toughness: 2 }` with `GlobalOther(And(You, HasSubtype("Vampire")))` -- correct for "+2/+2 to other Vampires you control".

## Behavior Checks
- **Tap ability (both faces):** Creates a 2/2 black Vampire creature token with flying. Correct.
- **Transform ability (front only):** Costs {B}, requires 5+ Vampires, only available when not transformed. Correct.
- **Transform execution:** Sets `is_transformed = true` and updates name. Correct.
- **Vampire counting:** Checks subtypes on both the object and via registry. Correct.

## Verdict: PASS
