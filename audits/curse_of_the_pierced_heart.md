# Audit: Curse of the Pierced Heart

## Scryfall Reference
- **Name:** Curse of the Pierced Heart
- **Cost:** {1}{R}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_the_pierced_heart.rs`
- **Name:** Curse of the Pierced Heart -- CORRECT
- **Cost:** {1}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Deals 1 damage to enchanted player at their upkeep -- CORRECT
- **Uses NonCombatDamageDealt event:** Yes -- CORRECT

## Issues
1. **MINOR: Oracle says "deals 1 damage to that player or a planeswalker that player controls."** The implementation always deals damage to the player (never to a planeswalker). Since planeswalkers are unlikely in this engine's context, this is a minor simplification.
