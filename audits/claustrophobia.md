# Audit: Claustrophobia

## Scryfall Reference
- **Name:** Claustrophobia
- **Cost:** {1}{U}{U}
- **Type:** Enchantment -- Aura
- **Oracle:** Enchant creature. When this Aura enters, tap enchanted creature. Enchanted creature doesn't untap during its controller's untap step.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `claustrophobia.rs`
- **Name:** Claustrophobia -- CORRECT
- **Cost:** {1}{U}{U} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Keywords:** none (Enchant not in keywords list) -- ACCEPTABLE
- **Continuous effect:** PreventUntap { scope: Attached } -- CORRECT
- **On resolve:** Taps enchanted creature + attaches -- CORRECT
- **Target:** TargetRequirement::Creature -- CORRECT

## Issues
None
