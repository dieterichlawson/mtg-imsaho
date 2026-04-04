## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
**Type line**: Creature — Spirit
**Status**: ISSUE

### Code issues
- Major conceptual bug at mtg-engine/src/cards/isd/dearly_departed.rs:30-35 and line 39
  - Oracle text says: `As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.`
  - Code does: Implements this as a triggered ability (`TriggerKind::AnyCreatureEnters` and `on_any_creature_enters`) when it should be a replacement effect. "As long as" indicates a continuous replacement effect that modifies how creatures enter, not a triggered ability that fires after they enter.

- Major engine bug at mtg-engine/src/triggers.rs:915 and mtg-engine/src/triggers.rs:369
  - Oracle text says: `As long as this creature is in your graveyard` (effect must work from graveyard)
  - Code does: Trigger system only scans objects `o.zone == Zone::Battlefield` for watchers and checks `state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield)` before dispatching. Dearly Departed in graveyard will never be found as a watcher and the trigger will never fire.

- Test inadequacy at mtg-engine/tests/tier15_cards.rs:62
  - Oracle text says: Should work through normal game mechanics
  - Code does: Test manually calls `behavior.on_any_creature_enters()` bypassing the trigger system, so it doesn't test whether the card actually works in practice.

### Tricky interactions checked
- "As long as" continuous vs triggered: FAIL (implemented as triggered ability instead of replacement effect)
- Graveyard location requirement: FAIL (trigger system can't find objects in graveyard)  
- Cumulative effects from multiple copies: PASS (would work if the mechanism worked, as each copy would add its own counter)
- Human subtype checking for tokens: PASS (correctly checks both registry and runtime subtypes)
- "You control" restriction: PASS (correctly checks entered_controller != owner)
- Multiple simultaneous creatures entering: UNCERTAIN (depends on how replacement effects are batched, but mechanism is wrong anyway)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic functionality (Human gets counter when Dearly Departed in graveyard): `tier15_cards.rs:49` / INADEQUATE (bypasses trigger system)
- Cumulative effects from multiple Dearly Departed: NOT TESTED
- Non-Human creatures don't get counters: NOT TESTED  
- Effect only works when Dearly Departed is in graveyard: NOT TESTED
- Effect only affects creatures you control: NOT TESTED
- Replacement effect timing (counters present when ETB triggers check): NOT TESTED