## Audit — 2026-04-01

**Scryfall Oracle text (front face)**: Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform Bloodline Keeper. Activate only if you control five or more Vampires.
**Scryfall Oracle text (back face — Lord of Lineage)**: Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.
**Scryfall type line**: Creature — Vampire // Creature — Vampire
**Status**: PASS

- Mana cost {2}{B}{B}: correct
- Front face 3/3: correct
- Back face 5/5: correct
- Subtype Vampire (both faces): correct
- Keyword Flying (both faces): correct
- Front face tap ability creates 2/2 black Vampire with flying: correct
- Transform ability costs {B}, requires 5+ Vampires: correct
- Back face ModifyPT +2/+2 for other Vampire creatures you control: correct (GlobalOther with You + HasSubtype("Vampire"))
- Back face also has the tap-to-create-token ability: correct
- Token creation uses correct subtypes and keywords: correct
- Test exists in tier15_cards.rs
