## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
**Type line**: Creature — Dragon
**Status**: ISSUE

### Code issues
- Battlefield check is too restrictive (mtg-engine/src/cards/isd/balefire_dragon.rs:40-42)
  - Oracle text says: `Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.`
  - Code does: Returns early if the Dragon is not on the battlefield when the trigger resolves, preventing the ability from working. According to MTG rules, triggered abilities should resolve using "last known information" even if the source leaves the battlefield. The search results specifically confirm this applies to Balefire Dragon.

### Tricky interactions checked
- Non-combat damage: PASS - Correctly uses `NonCombatDamageDealt` event type per the ruling
- Source leaves battlefield before trigger resolves: FAIL - Code prevents resolution, but should use last known information per MTG rules
- Triggers on actual combat damage dealt: PASS - Uses the amount parameter from combat damage event
- Only affects creatures controlled by damaged player: PASS - Correctly filters by `o.controller == damaged_player`
- Each creature (no targeting): PASS - Affects all creatures without targeting
- Mandatory effect: PASS - No "may" in oracle text, correctly implemented as mandatory

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic functionality (triggers and deals damage to opponent creatures): `tier6_cards.rs:329` 
- Does not affect own creatures: `tier6_cards.rs:329`
- Non-combat damage ruling: NOT TESTED
- Source leaves battlefield before resolution: NOT TESTED
- Double strike (should trigger twice): NOT TESTED  
- Trample (damage amount based on actual damage to player): NOT TESTED