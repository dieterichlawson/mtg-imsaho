## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Trample\nMorbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
**Type line**: Creature — Boar
**Status**: PASS

### Code issues

No issues found.

### Tricky interactions checked
- Morbid condition tracking (creature_died_this_turn flag): PASS - Flag is properly set when creatures die (sba.rs:96,144, destruction.rs:100) and reset at turn start (engine.rs:2888)
- Token creature deaths enabling morbid: PASS - Token creatures die and go to graveyard briefly before ceasing to exist, setting creature_died_this_turn flag
- Replacement effect timing ("enters with" vs "when enters"): PASS - Uses on_resolve pattern which is engine's standard for "enters with" effects, functionally equivalent to replacement effect
- Morbid persistent throughout turn: PASS - Flag remains true once set until start of next turn
- Multiple creature deaths in same turn: PASS - Flag remains true regardless of how many creatures died
- Keywords vs ability words: PASS - Morbid correctly excluded from keywords vector (it's an ability word, not keyword)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Morbid active case (2 +1/+1 counters): `mtg-engine/tests/tier5_cards.rs:217` / TESTED
- No morbid case (0 counters): `mtg-engine/tests/tier5_cards.rs:234` / TESTED  
- Creature death tracking mechanics: `mtg-engine/tests/card_mechanics.rs:28` / TESTED
- Token creature deaths enabling morbid: NOT DIRECTLY TESTED (tested via other morbid cards)
- Morbid flag reset at turn boundaries: `mtg-engine/tests/card_mechanics.rs:41` / TESTED
- Multiple simultaneous creature deaths: NOT DIRECTLY TESTED (implied by flag persistence)
