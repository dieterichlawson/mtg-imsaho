# Audit: Curse of the Bloody Tome

## Scryfall Reference
- **Name:** Curse of the Bloody Tome
- **Cost:** {2}{U}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. At the beginning of enchanted player's upkeep, that player mills two cards.
- **P/T:** N/A
- **Keywords:** Enchant, Mill

## Implementation: `curse_of_the_bloody_tome.rs`
- **Name:** Curse of the Bloody Tome -- CORRECT
- **Cost:** {2}{U} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Mills 2 cards at enchanted player's upkeep using mill_cards -- CORRECT

## Issues
None
