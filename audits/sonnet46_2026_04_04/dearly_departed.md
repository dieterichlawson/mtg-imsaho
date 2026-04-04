## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
**Type line**: Creature — Spirit
**Status**: ISSUE

### Code issues

- **Engine: `AnyCreatureEnters` watcher scan only checks `Zone::Battlefield`; Dearly Departed in the graveyard is never dispatched a trigger** (`mtg-engine/src/triggers.rs:368-369`)
  - Oracle text says: `"As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it."`
  - Code does: `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != *object)` — the watcher collection for `AnyCreatureEnters` exclusively scans `Zone::Battlefield`. Dearly Departed resides in `Zone::Graveyard` when its ability is active, so it is never included in the watcher list and its `EnterWatch` trigger is never queued. The ability cannot fire.

- **Engine: `EnterWatch` trigger resolution also requires watcher to be on `Zone::Battlefield`** (`mtg-engine/src/triggers.rs:914-915`)
  - Oracle text says: `"As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it."`
  - Code does: `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` — even if an `EnterWatch` trigger for Dearly Departed were somehow queued, the resolve handler would silently skip it because it guards on `zone == Zone::Battlefield`. Dearly Departed is in the graveyard, so the handler body is never entered.

### Tricky interactions checked

- **Graveyard watcher trigger dispatch**: FAIL — `triggers.rs:368-369` filters watchers to `Zone::Battlefield` only; graveyard objects are never considered.
- **Trigger resolution guard for EnterWatch**: FAIL — `triggers.rs:915` also requires `zone == Zone::Battlefield`, providing a second barrier preventing the ability from resolving even if the first bug were patched independently.
- **Human subtype check covers both registry and runtime (token) subtypes**: PASS — card code at `dearly_departed.rs:52-58` checks both `registry.card_data(cid).map(|d| d.subtypes…)` and `state.get_object(entered_id).map(|o| o.subtypes…)`, matching the pattern in `state.rs:check_condition`.
- **Controller/owner check for "you control" vs graveyard ownership**: PASS — `dearly_departed.rs:44-48` correctly uses `self_obj.owner` (graveyard objects belong to their owner) and compares against `entered_controller`, matching the oracle's "you control" restriction.
- **Multiple Dearly Departed cumulative counter ruling**: NOT TESTED via engine — the ruling says "the effect is cumulative" (one counter per Dearly Departed in graveyard). Since each Dearly Departed would be a separate graveyard watcher that fires independently, this would be correct if the dispatch bug were fixed, but it is untestable in the current engine.
- **Non-Human creature enters — no counter added**: PASS (in handler logic) — `dearly_departed.rs:59-61` correctly guards on `is_human` before calling `add_counters`.
- **Opponent's Human creature enters — no counter added**: PASS (in handler logic) — `dearly_departed.rs:47-49` returns early when `entered_controller != owner`.

### Test coverage

- **Graveyard-based trigger fires through full engine (trigger dispatch → collect → resolve)**: NOT TESTED — the only test (`tier15_cards.rs:49-67`) directly calls `behavior.on_any_creature_enters(...)`, bypassing the trigger collection and dispatch path entirely. It proves the card handler logic is correct but does not catch the dispatch bug.
- **Multiple Dearly Departed in graveyard — two counters placed**: NOT TESTED
- **Human token entering under your control gets counter**: NOT TESTED (token subtype path)
- **Non-Human entering — no counter**: NOT TESTED via engine dispatch
- **Opponent's Human entering — no counter**: NOT TESTED via engine dispatch
- **Cumulative ruling (one counter per Dearly Departed in graveyard)**: NOT TESTED
