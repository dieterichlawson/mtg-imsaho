## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, target creature can't block this turn.
**Type line**: Creature — Vampire
**Status**: ISSUE

### Code issues
- Engine bug in trigger resolution prevents ETB triggers from resolving if source leaves battlefield (mtg-engine/src/triggers.rs:895)
  - Oracle text says: `When this creature enters, target creature can't block this turn.`
  - Code does: Checks if source is still on battlefield before resolving ETB trigger, preventing resolution if source has left battlefield. MTG rules state triggered abilities should resolve independently of their source.

### Tricky interactions checked
- Targeting self: PASS - code correctly allows targeting any creature including Crossway Vampire itself
- No creatures available: PASS - present_target_choice handles empty target list correctly 
- Source leaves battlefield before trigger resolves: FAIL - engine incorrectly prevents trigger resolution
- Effect cleanup at end of turn: PASS - until_end_of_turn_cant_block is properly cleared in cleanup step
- Effect enforcement during combat: PASS - combat.rs correctly filters creatures that can't block

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic ETB targeting functionality: NOT TESTED
- Targeting self vs other creatures: NOT TESTED  
- Source leaving battlefield before trigger resolution: NOT TESTED
- Effect lasting until end of turn: NOT TESTED
- Effect preventing blocking during combat: NOT TESTED