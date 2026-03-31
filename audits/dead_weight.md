# Audit: Dead Weight

## Scryfall Reference
- **Name:** Dead Weight
- **Cost:** {B}
- **Type:** Enchantment -- Aura
- **Oracle:** Enchant creature. Enchanted creature gets -2/-2.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `dead_weight.rs`
- **Name:** Dead Weight -- CORRECT
- **Cost:** {B} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Continuous effect:** ModifyPT { power: -2, toughness: -2, scope: Attached } -- CORRECT
- **Target:** TargetRequirement::Creature -- CORRECT

## Issues
None
