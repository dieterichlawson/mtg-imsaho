## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
**Type line**: Creature — Zombie
**Status**: ISSUE

### Code issues
- Source permanent zone requirement (abattoir_ghoul.rs:39-42)
  - Oracle text says: `Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.`
  - Code does: Requires the Abattoir Ghoul to still be on the battlefield when the damaged creature dies (`Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return`). This contradicts general MTG rules that triggered abilities exist independently once on the stack and should resolve using last-known information.

### Tricky interactions checked
- Source leaves battlefield before damaged creature dies: FAIL - trigger doesn't fire when it should per general MTG rules
- Last-known toughness calculation: PASS - correctly uses `dead_toughness` parameter
- First strike damage tracking: PASS - damage tracking system works with first strike creatures
- Damage tracking across turn: PASS - `damaged_by` field correctly tracks and clears at end of turn
- Multiple simultaneous creature deaths: PASS - trigger system handles batch processing correctly
- "This turn" timing window: PASS - damage tracking is cleared at end of turn

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic life gain from damaged creature death: `tier6_cards.rs:20`
- No life gain if creature not damaged by Ghoul: `tier6_cards.rs:43`
- Last-known toughness with +1/+1 counters: `tier6_cards.rs:61`
- Source permanent leaves battlefield before victim dies: NOT TESTED
- First strike damage timing: NOT TESTED
- Multiple creatures dying simultaneously: NOT TESTED
- Zero or negative toughness handling: NOT TESTED