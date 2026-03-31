# Audit: Nevermore

## Official Oracle
- **Name:** Nevermore
- **Cost:** {1}{W}{W}
- **Type:** Enchantment
- **Oracle:** As Nevermore enters the battlefield, choose a nonland card name. Spells with the chosen name can't be cast.

## Implementation: `mtg-engine/src/cards/nevermore.rs`
- **Name:** Nevermore -- CORRECT
- **Cost:** {1}{W}{W} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **on_enter_battlefield:** Auto-selects a nonland card name from opponent's hand -- CORRECT concept

## Issues
1. **Name selection too narrow:** The oracle lets you name ANY nonland card (even one not in the game). The implementation only looks at the opponent's hand and defaults to "Lightning Bolt" if nothing found. In a real game, you'd typically name a card you expect the opponent to have. The auto-selection from opponent's hand is a reasonable simplification, but the "Lightning Bolt" fallback is odd since it's not in Innistrad.

## Verdict
**PASS** -- The auto-selection is a reasonable AI simplification. The "Lightning Bolt" default is cosmetically odd but functionally harmless.
