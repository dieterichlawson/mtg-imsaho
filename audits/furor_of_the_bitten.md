# Audit: Furor of the Bitten

## Reference (Scryfall)
- **Name:** Furor of the Bitten
- **Cost:** {R}
- **Type:** Enchantment -- Aura
- **Oracle:** Enchant creature. Enchanted creature gets +2/+2 and attacks each combat if able.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({R})
- Type: CORRECT (Enchantment)
- Subtypes: CORRECT (Aura)
- Oracle text: CORRECT
- P/T: CORRECT (N/A)
- +2/+2 to enchanted creature: CORRECT (ModifyPT power:2, toughness:2, scope: Attached)
- Attacks each combat if able: CORRECT (ForceAttack, scope: Attached)
- Target requirement: CORRECT (Creature)
- Resolves as aura: CORRECT (resolve_aura)

## Issues
None found.
