# Audit: Curse of the Nightly Hunt

## Scryfall Reference
- **Name:** Curse of the Nightly Hunt
- **Cost:** {2}{R}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. Creatures enchanted player controls attack each combat if able.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_the_nightly_hunt.rs`
- **Name:** Curse of the Nightly Hunt -- CORRECT
- **Cost:** {2}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Continuous effect:** ForceAttack { scope: Global(AttachedPlayer) } -- CORRECT

## Issues
None
