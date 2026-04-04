## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
**Type line**: Legendary Creature — Spirit Cleric
**Status**: ISSUE

### Code issues
- Target selection for Angel token (mtg-engine/src/cards/isd/geist_of_saint_traft.rs:74-77)
  - Oracle text says: `You choose which player or planeswalker the Angel token is attacking. It doesn't have to be attacking the same player or planeswalker that Geist of Saint Traft is attacking.` (from Scryfall ruling 2020-08-07)
  - Code does: `let defender = state.opponent(controller); combat.attackers.insert(token_id, defender);` - automatically targets the opponent with no player choice and no support for planeswalker targets

### Tricky interactions checked
- Angel token enters attacking without being declared as attacker: pass - code correctly adds to combat.attackers without going through declare attackers step
- Angel exiles even if Geist dies: pass - uses state.end_of_combat_exiles which is independent of source
- Token creation is mandatory (no "may"): pass - code creates token without optional choice
- Hexproof implementation: pass - correctly declared in keywords vec
- Token stats (4/4 white Angel with flying): pass - create_token_with_subtypes called with correct parameters
- "Whenever attacks" triggers only for declared attackers: pass - Angel bypasses trigger system by entering already attacking
- Multiple tokens from Doubling Season all exiled: pass - all tokens stored in end_of_combat_exiles vector

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic token creation on attack: `mtg-engine/tests/geist_of_saint_traft.rs:19` 
- Angel exiled at end of combat: `mtg-engine/tests/geist_of_saint_traft.rs:44`
- Angel exiled even if Geist dies: `mtg-engine/tests/geist_of_saint_traft.rs:70`
- Player choice for Angel token target: NOT TESTED
- Angel attacking different target than Geist: NOT TESTED
- Angel attacking planeswalker: NOT TESTED
- Multiple Angel tokens from doubling effects: NOT TESTED
- Angel bypassing "can't attack" effects: NOT TESTED
- Angel not triggering "whenever attacks" abilities: NOT TESTED