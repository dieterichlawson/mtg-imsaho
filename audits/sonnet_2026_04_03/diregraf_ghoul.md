## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature enters tapped.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Clone effects that copy Diregraf Ghoul: pass (tapped state set correctly on object after entry)
- Effects that untap creatures as they enter battlefield: pass (replacement effect happens first)
- Mass reanimation or token creation effects: pass (each object gets tapped individually)
- Flickering/bouncing effects: pass (re-entry triggers the enters tapped effect again)
- Grimoire of the Dead or similar mass battlefield entry: pass (standard battlefield entry mechanics)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Creature enters tapped when cast normally: `mtg-engine/tests/innistrad_cards.rs:141` (diregraf_ghoul_enters_tapped)
- Clone effects copying enters-tapped property: NOT TESTED
- Interaction with untap effects during resolution: NOT TESTED  
- Mass battlefield entry scenarios: NOT TESTED
- Flicker/bounce re-entry scenarios: NOT TESTED