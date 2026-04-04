## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
**Type line**: Creature — Wolf
**Status**: ISSUE

### Code issues
- Once-per-turn restriction never resets between turns at `mtg-engine/src/engine.rs:2911-2954` and `mtg-engine/src/engine.rs:3006-3034`
  - Oracle text says: `Activate only once each turn.`
  - Code does: Tracks activations in `abilities_activated_this_turn` HashSet but never clears this set during turn transitions. The set is added to on activation (line 1778) and checked for restriction (line 358), but is never cleared in cleanup step or untap step, making the restriction permanent after first use.

### Tricky interactions checked
- Until end of turn cleanup: pass (properly cleared in cleanup step at line 3021)
- Once per turn restriction within turn: pass (correctly prevents second activation)
- Once per turn restriction across turns: fail (never resets, becomes permanent)
- Activated ability targeting: pass (affects "this creature" correctly with no target requirement)
- Mana cost and effect values: pass (matches {2}{G} cost and +2/+2 effect)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic +2/+2 effect: `mtg-engine/tests/activated_abilities.rs:190`
- Once per turn within same turn: `mtg-engine/tests/activated_abilities.rs:210`
- Once per turn resets on new turn: NOT TESTED
- Until end of turn cleanup: NOT TESTED for this specific card
- Correct mana cost and stats: `mtg-engine/tests/activated_abilities.rs:180`

Sources:
- [Darkthicket Wolf · Innistrad (ISD) #175 - Scryfall](https://scryfall.com/card/isd/175/darkthicket-wolf)
- [Does "Once per turn" really mean only once? — MTG Q&A](https://tappedout.net/mtg-questions/does-once-per-turn-really-mean-only-once/)