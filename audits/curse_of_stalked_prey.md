# Audit: Curse of Stalked Prey

## Scryfall Reference
- **Name:** Curse of Stalked Prey
- **Cost:** {1}{R}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_stalked_prey.rs`
- **Name:** Curse of Stalked Prey -- CORRECT
- **Cost:** {1}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Trigger:** AnyCombatDamageToPlayer -- CORRECT
- **Behavior:** Adds +1/+1 counter when creature deals combat damage to enchanted player -- CORRECT

## Issues
None
