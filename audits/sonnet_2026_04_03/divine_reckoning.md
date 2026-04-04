## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Each player chooses a creature they control. Destroy the rest.
Flashback {5}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Turn order for choices**: pass - Code correctly implements the 2011-09-22 ruling by rotating player order so active player is first (lines 43-46)
- **Auto-handling for 0-1 creatures**: pass - Players with 0 or 1 creatures automatically have their choice resolved without presenting UI (lines 58-68)
- **"Each player chooses" vs targeting**: pass - No targeting involved, each player makes their own choice in sequence
- **"Destroy the rest" with indestructible**: pass - Uses `try_destroy` which correctly checks for Indestructible keyword and prevents destruction
- **Flashback exile after resolution**: pass - Uses `move_spell_after_resolve` which checks `cast_with_flashback` flag and moves to exile instead of graveyard
- **Creature filtering logic**: pass - Correctly filters by `controller == player_id && power.is_some()` to identify creatures controlled by each player
- **Sequential choice presentation**: pass - Uses `KeepOneDestroyRest` pending effect to handle choices one player at a time in turn order
- **Spell cleanup timing**: pass - Calls `move_spell_after_resolve` before presenting choices (line 71), ensuring spell is properly handled regardless of choice outcomes

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic multi-player scenario with choices**: `tier8_cards.rs:255-310` (divine_reckoning_keeps_one_per_player)
- **Auto-keep for single creature**: `tier8_cards.rs:313-333` (divine_reckoning_with_one_creature_keeps_it)  
- **Flashback cost verification**: `tier8_cards.rs:336-342` (divine_reckoning_has_flashback)
- **Turn order implementation**: NOT TESTED - No test verifies active player chooses first
- **Indestructible interaction**: NOT TESTED - No test verifies indestructible creatures survive
- **Zero creatures scenario**: NOT TESTED - No test verifies behavior when a player controls no creatures
- **Flashback exile after resolution**: NOT TESTED - No specific test for Divine Reckoning flashback resolution cleanup