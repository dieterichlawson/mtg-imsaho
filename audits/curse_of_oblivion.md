# Audit: Curse of Oblivion

## Scryfall Reference
- **Name:** Curse of Oblivion
- **Cost:** {3}{B}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_oblivion.rs`
- **Name:** Curse of Oblivion -- CORRECT
- **Cost:** {3}{B} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Cursed player exiles two cards from graveyard at their upkeep -- CORRECT
- **Choice mechanism:** Presents choice if >2 cards, auto-exiles if <=2 -- CORRECT

## Issues
None
