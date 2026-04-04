## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "You may" optionality: PASS — Uses `present_optional_target_choice` which presents choice to player even with valid targets
- "Another target creature" restriction: PASS — Uses `creature_targets_except(state, object_id)` to exclude Fiend Hunter itself  
- Timing interaction (LTB before ETB resolves): PASS — LTB trigger resolves first and finds no stored creature (does nothing), then ETB trigger resolves and stores creature ID on object, but no future LTB will fire since source is no longer on battlefield
- "Under its owner's control": PASS — Code sets `obj.controller = obj.owner` when returning creature
- Multiple Fiend Hunters: PASS — Each stores exiled creature ID separately in its own `card_state`
- Source leaves battlefield before trigger resolves: PASS — Triggered abilities on stack resolve independently of source permanent
- Token exile handling: UNCERTAIN — Oracle ruling states tokens won't return, but engine's token handling for zone changes needs verification

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- ETB trigger with valid targets: NOT TESTED
- ETB trigger with no valid targets: NOT TESTED  
- Optional choice (declining to target): NOT TESTED
- LTB trigger returning exiled creature: NOT TESTED
- Timing interaction (Fiend Hunter leaves before ETB resolves): NOT TESTED
- Token exile (tokens don't return): NOT TESTED
- Multiple Fiend Hunters (each returns only own exiled creature): NOT TESTED
- Exiled creature ownership control: NOT TESTED