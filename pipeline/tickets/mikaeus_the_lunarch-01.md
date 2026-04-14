---
id: mikaeus_the_lunarch-01
status: new
card: Mikaeus, the Lunarch
card_file: mtg-engine/src/cards/isd/mikaeus_the_lunarch.rs
created: 2026-04-14T20:45:48Z
audit_run_id: 2026-04-14-mikaeus_the_lunarch-audit
audit_model: opus
audit_tokens: 19324
audit_duration: 400
---

## Audit Finding

**Oracle text:**
> Mikaeus enters with X +1/+1 counters on it.

**Code:**
> `state.move_object(object_id, Zone::Battlefield, registry);`
> `let x = state.get_object(object_id).and_then(|o| o.x_value).unwrap_or(0);`
> `state.add_counters(object_id, CounterType::PlusOnePlusOne, x);`
> — mikaeus_the_lunarch.rs:38-42

**Description:**
"Enters with X +1/+1 counters" is a replacement effect per CR 614.1c — it modifies how the permanent enters the battlefield. The engine implements this via `entering_with_counters` (state.rs:716), which is called inside `compute_entering_counters` (state.rs:701) BEFORE the zone change, and the counters are applied BEFORE the `EnteredBattlefield` event fires (state.rs:612-621). Other "enters with" cards use this mechanism: Unbreathing Horde (unbreathing_horde.rs:50), Festerhide Boar (festerhide_boar.rs:33), Somberwald Spider (somberwald_spider.rs:30). Mikaeus instead adds counters manually in `on_resolve` AFTER `move_object` returns — meaning counters are added AFTER the EnteredBattlefield event. Because the engine processes trigger events asynchronously (events are pushed to a queue at state.rs:618, processed later during trigger dispatch), this currently produces correct observable behavior. However, the code bypasses the replacement-effect pipeline, meaning any future interaction that modifies entering counters (e.g., Doubling Season, Vorinclex) through the `compute_entering_counters` pipeline would not apply to Mikaeus' X counters.

**Engine path:**
- mikaeus_the_lunarch.rs:38 (`move_object` call)
- mikaeus_the_lunarch.rs:40-42 (manual counter addition after move)
- state.rs:558-562 (`compute_entering_counters` called during `move_object` — returns empty for Mikaeus)
- state.rs:612-615 (entering counters applied before ETB event)
- state.rs:701-736 (`compute_entering_counters` — the canonical entering-counter pipeline)

**Required check:** 8a (zone-change entering procedure)

**Affected cards:**
- Mikaeus, the Lunarch

