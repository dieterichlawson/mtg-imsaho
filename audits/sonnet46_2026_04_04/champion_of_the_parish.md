## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever another Human you control enters, put a +1/+1 counter on this creature.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Another" constraint (self-exclusion)**: The `collect_triggers` dispatch at `triggers.rs:369` uses `o.id != *object` to filter the watcher scan, so the Champion cannot watch its own entry. No explicit `entered_id != self_id` check is needed in the handler — the dispatch guarantees it. PASS
- **"You control" constraint**: `on_any_creature_enters` at `champion_of_the_parish.rs:42` checks `entered_controller != controller` before adding the counter. PASS
- **Human subtype check — registry path (non-token creatures)**: `champion_of_the_parish.rs:47-50` checks `registry.card_data(cid).map(|d| d.subtypes.iter().any(|s| s == "Human"))`. PASS
- **Human subtype check — object subtypes path (tokens)**: `champion_of_the_parish.rs:51-53` also checks `o.subtypes.iter().any(|s| s == "Human")` directly on the runtime object. Human tokens store their subtype on the object, not in the registry (card_id is sentinel 0), so this correctly handles Human tokens. PASS
- **Champion must still be on battlefield at resolution**: Trigger resolution in `triggers.rs:914-919` first verifies `watcher_id` is still on the battlefield; `on_any_creature_enters` also re-checks at `champion_of_the_parish.rs:37-40`. PASS
- **Trigger dispatch guards — `!desc.is_empty()` check**: The ETB-watch scan at `triggers.rs:375` only creates an `EnterWatch` trigger when the watcher's `AnyCreatureEnters` description is non-empty. Champion's description is `"put a +1/+1 counter on Champion of the Parish"` (non-empty), so the trigger is always collected. PASS
- **Entering object must be a creature**: `triggers.rs:367` gates the ETB-watch scan on `o.power.is_some()`, correctly restricting the trigger to creatures entering. PASS
- **Multiple Champions trigger each other**: If Champion A is on the battlefield and Champion B enters, Champion A watches Champion B's entry (different ObjectIds). The "another" constraint is satisfied. PASS
- **Trigger fires once per Human entering**: Each `EnteredBattlefield` event is processed separately by the event loop in `collect_triggers`, so multiple Humans entering in a single batch each generate a separate `EnterWatch` trigger. PASS
- **Counter type**: `CounterType::PlusOnePlusOne` is used at `champion_of_the_parish.rs:55`, matching the oracle text "+1/+1 counter". PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Another Human you control entering puts a +1/+1 counter on Champion: `tier6_cards.rs:87` (`champion_of_the_parish_counter_on_human_etb`) TESTED
- Non-Human creature entering does not trigger Champion: `tier6_cards.rs:108` (`champion_of_the_parish_no_counter_on_non_human`) TESTED
- Opponent's Human entering does not trigger Champion: `tier6_cards.rs:129` (`champion_of_the_parish_no_counter_on_opponent_human`) TESTED
- Human token entering triggers Champion: NOT TESTED
- Champion entering the battlefield does not give itself a counter: NOT TESTED
- Two Champions triggering each other when one enters: NOT TESTED
- Multiple Humans entering simultaneously (one counter per Human): NOT TESTED
- Champion leaving battlefield before trigger resolves (counter not added): NOT TESTED
