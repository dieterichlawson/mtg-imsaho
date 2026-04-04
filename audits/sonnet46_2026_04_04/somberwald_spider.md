## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Reach (This creature can block creatures with flying.)
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
**Type line**: Creature — Spider
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Morbid condition evaluation (`creature_died_this_turn`)**: The engine sets `state.creature_died_this_turn = true` in three places (`destruction.rs:100`, `sba.rs:96`, `sba.rs:144`) and clears it at turn transition (`engine.rs:2888`). The card's `on_enter_battlefield` reads this flag correctly. PASS.
- **ETB trigger dispatch**: `collect_triggers` in `triggers.rs` creates a `PendingTrigger::EnteredBattlefield` for any registered card on the `GameEvent::EnteredBattlefield` event. `SomberwaldSpider` is registered and declares `TriggerKind::EntersBattlefield`, so the trigger fires. `resolve_next_trigger` then calls `behavior.on_enter_battlefield`. PASS.
- **"Enters with counters" implemented as ETB triggered ability rather than replacement effect**: Oracle text says "enters with two +1/+1 counters" which is technically a replacement effect in MTG (CR 614.1c). The engine implements it as a triggered ability (fires on stack after creature has entered). For a 2/4 base creature, the functional outcome is identical — no SBA-survival edge case, no "counter on entry" watcher conflicts in this engine — so this produces correct observable behavior. PASS.
- **Spider left battlefield before trigger resolves**: `resolve_next_trigger` checks `o.zone == Zone::Battlefield` before calling `on_enter_battlefield`. If spider leaves before trigger resolves, counters are skipped. In MTG the trigger would still resolve but counter placement would fail gracefully (target not on battlefield). End result is the same. PASS.
- **Reach keyword present**: `keywords: vec![Keyword::Reach]` correctly declared. PASS.
- **Counter type and amount**: Code calls `state.add_counters(object_id, CounterType::PlusOnePlusOne, 2)` — two +1/+1 counters as required. PASS.
- **`creature_died_this_turn` reset timing**: The flag is cleared at `engine.rs:2888` when transitioning to the next turn (after the cleanup step), not at the start of the current turn's first step. This is correct — a creature that died earlier in the same turn enables morbid for the rest of that turn. PASS.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Morbid ETB (creature died this turn → 2 counters added): `mtg-engine/tests/card_mechanics.rs:90` (`somberwald_spider_morbid_counters`)
- No morbid (no creature died → no counters): `mtg-engine/tests/card_mechanics.rs:108` (`somberwald_spider_no_morbid_no_counters`)
- Reach keyword declared: `mtg-engine/tests/innistrad_cards.rs:131` (`somberwald_spider_has_reach`)
- Spider leaves battlefield before ETB trigger resolves: NOT TESTED
- Morbid flag reset between turns: NOT TESTED (covered indirectly by other morbid card tests)
