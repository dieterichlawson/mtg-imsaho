# Audit: Fiend Hunter

## Reference (Scryfall)
- **Name:** Fiend Hunter
- **Cost:** {1}{W}{W}
- **Type:** Creature -- Human Cleric
- **Oracle:** When Fiend Hunter enters the battlefield, you may exile another target creature. When Fiend Hunter leaves the battlefield, return the exiled card to the battlefield under its owner's control.
- **P/T:** 1/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{W}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Cleric)
- Oracle text: CORRECT
- P/T: CORRECT (1/3)
- ETB trigger: CORRECT (TriggerKind::EntersBattlefield)
- LTB trigger: CORRECT (TriggerKind::LeavesBattlefield)
- "You may" optional exile: CORRECT (uses present_optional_target_choice)
- "Another target creature": CORRECT (creature_targets_except excludes self)
- Returns exiled card on LTB: CORRECT (checks Zone::Exile, moves to Battlefield)

## Issues
None found.
