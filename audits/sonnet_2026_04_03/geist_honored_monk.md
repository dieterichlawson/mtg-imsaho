## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Vigilance
Geist-Honored Monk's power and toughness are each equal to the number of creatures you control.
When this creature enters, create two 1/1 white Spirit creature tokens with flying.
**Type line**: Creature — Human Monk
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Self-counting**: PASS - dynamic_pt function counts all creatures you control including itself when on battlefield
- **All zones characteristic-defining**: PASS - dynamic_pt correctly determines P/T based on current battlefield state, works in all zones per rulings
- **Token creation timing**: PASS - ETB trigger creates 2 tokens correctly which then increase monk's own P/T
- **Token subtypes**: PASS - tokens created with correct subtypes "Spirit" and flying keyword
- **Creature counting filter**: PASS - dynamic_pt correctly filters by zone==Battlefield, controller match, and power.is_some()

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic dynamic P/T functionality**: `mtg-engine/tests/tier5_cards.rs:73-98` - Tests monk + 2 tokens = 3/3 P/T
- **Token creation**: `mtg-engine/tests/tier5_cards.rs:73-98` - Tests 2 Spirit tokens are created
- **Self-counting when on battlefield**: `mtg-engine/tests/tier5_cards.rs:73-98` - Covered indirectly in P/T calculation
- **All zones characteristic-defining**: NOT TESTED
- **Vigilance keyword**: NOT TESTED