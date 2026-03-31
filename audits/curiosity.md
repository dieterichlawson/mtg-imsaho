# Audit: Curiosity

## Scryfall Reference
- **Name:** Curiosity
- **Cost:** {U}
- **Type:** Enchantment -- Aura
- **Oracle:** Enchant creature. Whenever enchanted creature deals damage to an opponent, you may draw a card.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curiosity.rs`
- **Name:** Curiosity -- CORRECT
- **Cost:** {U} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::Creature -- CORRECT
- **Trigger:** AnyDamageToPlayer -- CORRECT
- **Behavior:** Draws a card when enchanted creature deals damage to opponent -- CORRECT
- **Checks:** source_id == attached_to, damaged_player != controller -- CORRECT

## Issues
None (the oracle says "damage" not "combat damage", and the implementation correctly triggers on any damage to an opponent, which is correct)
