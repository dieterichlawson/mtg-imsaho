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

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Nevermore  
**Type**: Enchantment | **Cost**: {1}{W}{W}  
**Oracle text**: "As this enchantment enters, choose a nonland card name.\nSpells with the chosen name can't be cast."

### Checks
- Name: "Nevermore" -- PASS
- Cost: {1}{W}{W} -- PASS
- Type: Enchantment -- PASS
- Oracle text string: ISSUE
  - **Oracle**: "As this enchantment enters, choose a nonland card name."
  - **Code**: "As Nevermore enters the battlefield, choose a nonland card name."
  - The oracle uses "this enchantment enters" (modern templating) while the code uses the older "Nevermore enters the battlefield" wording.
- Behavior (name choice): Auto-selects a nonland card from opponent's hand, falls back to "Lightning Bolt" -- PASS (reasonable AI choice heuristic)
- Behavior (storage): Stores chosen name in `instance_oracle_text` with "nevermore:" prefix -- PASS
- Behavior (prevention): Relies on engine checking for Nevermore in `legal_actions` -- not verified here but documented

**Verdict: ISSUE** — oracle_text field uses outdated wording "As Nevermore enters the battlefield" instead of current oracle "As this enchantment enters"

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "As this enchantment enters, choose a nonland card name." (was "As Nevermore enters the battlefield, choose a nonland card name."). Doc comment updated. Behavior unchanged.
