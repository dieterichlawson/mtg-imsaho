# Audit: Bloodline Keeper // Lord of Lineage

## Oracle (Scryfall/API)
### Front: Bloodline Keeper
- **Name:** Bloodline Keeper
- **Cost:** {2}{B}{B}
- **Type:** Creature — Vampire
- **Oracle:** Flying. {T}: Create a 2/2 black Vampire creature token with flying. {B}: Transform Bloodline Keeper. Activate only if you control five or more Vampires.
- **P/T:** 3/3

### Back: Lord of Lineage
- **Type:** Creature — Vampire
- **Oracle:** Flying. Other Vampire creatures you control get +2/+2. {T}: Create a 2/2 black Vampire creature token with flying.
- **P/T:** 5/5

## Implementation: `mtg-engine/src/cards/bloodline_keeper.rs`
- **Name:** Bloodline Keeper -- CORRECT
- **Cost:** {2}{B}{B} -- CORRECT
- **Type:** Creature — Vampire -- CORRECT
- **P/T:** 3/3 front, 5/5 back -- CORRECT
- **Keywords:** Flying (both faces) -- CORRECT
- **Token creation:** 2/2 black Vampire with flying -- CORRECT
- **Transform condition:** 5+ Vampires, costs {B} -- CORRECT
- **Back face continuous effect:** ModifyPT +2/+2 for other Vampires you control -- CORRECT
- **Vampire counting:** Checks both object subtypes and registry subtypes -- CORRECT
- **DFC handling:** Uses `back_face_data`, `dynamic_pt`, `is_transformed` -- CORRECT

## Verdict: PASS -- No issues found
