# Audit: Back from the Brink

## Oracle Text (Scryfall)
- **Name:** Back from the Brink
- **Mana Cost:** {4}{U}{U}
- **Type:** Enchantment
- **Oracle Text:** Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.

## Implementation File
`mtg-engine/src/cards/isd/back_from_the_brink.rs`

## Card Data Checks
- **Name:** Correct
- **Mana Cost:** Correct ({4}{U}{U})
- **Card Types:** Correct (Enchantment)
- **Oracle Text:** Correct

## Behavior Checks
- **Sorcery speed only:** Correct (`sorcery_speed_only: true`)
- **Creates token copy:** Correct (calls `create_token_copy`)
- **Exiles creature from graveyard:** Correct (moves to Exile zone)

### ISSUE: Activation cost is wrong
- **Oracle:** "Exile a creature card from your graveyard **and pay its mana cost**" -- the cost is the exiled card's mana cost, which varies per creature.
- **Implementation:** Uses a flat `Generic(2)` as the activation cost (line 58-59), which is incorrect. The comment acknowledges this: "The mana cost requirement is approximated by a high generic cost" but {2} is not even high.

### ISSUE: No player choice for which creature to exile
- **Oracle:** The player chooses which creature card to exile from their graveyard.
- **Implementation:** Automatically picks "the first creature in graveyard" (line 75-78) with no player choice.

## Verdict: ISSUE
- Activation cost is hardcoded to {2} instead of paying the exiled card's mana cost.
- No player choice for which creature to exile from graveyard.
