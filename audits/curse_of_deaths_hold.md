# Audit: Curse of Death's Hold

## Scryfall Reference
- **Name:** Curse of Death's Hold
- **Cost:** {3}{B}{B}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. Creatures enchanted player controls get -1/-1.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_deaths_hold.rs`
- **Name:** Curse of Death's Hold -- CORRECT
- **Cost:** {3}{B}{B} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Continuous effect:** ModifyPT { power: -1, toughness: -1, scope: Global(AttachedPlayer) } -- CORRECT

## Issues
None
