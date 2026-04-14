---
id: heartless_summoning-01
status: new
card: Heartless Summoning
card_file: mtg-engine/src/cards/isd/heartless_summoning.rs
created: 2026-04-14T21:29:30Z
audit_run_id: 2026-04-14-heartless_summoning-audit
audit_model: opus
audit_tokens: 19813
audit_duration: 410
---

## Audit Finding

**Oracle text:**
> Creature spells you cast cost {2} less to cast.

**Code:**
> engine.rs:2219-2227 — flashback cast path uses raw flashback cost without calling `effective_spell_cost`:
> ```
> } else if is_flashback {
>     let dynamic_fb = new_state.until_end_of_turn.iter()
>         .find_map(|e| if let ... GrantFlashback { target, cost } = e {
>             if *target == *object_id { Some(cost.clone()) } else { None }
>         } else { None });
>     dynamic_fb.unwrap_or_else(|| {
>         data.flashback_cost.expect("flashback cast on card without flashback_cost")
>     })
> ```
> The same omission exists in legal_actions at engine.rs:1231-1245 where `fb_cost` is the raw flashback/dynamic cost, never passed through `effective_spell_cost`.

**Description:**
Per CR 601.2f, the total cost of a spell is "the mana cost or alternative cost (as determined in rule 601.2b), plus all additional costs and cost increases, and minus all cost reductions." Flashback is an alternative cost (CR 702.33a). Cost reductions like Heartless Summoning's apply AFTER the alternative cost is selected. The engine's flashback paths — both in `legal_actions` (affordability/display) and `submit_action` (execution) — use the raw flashback cost without applying `effective_spell_cost`, so Heartless Summoning's {2} reduction never applies to creature spells cast via flashback. No creature in ISD has native flashback, but the CR-defined procedure is violated, and dynamically granted flashback (e.g., via Snapcaster Mage targeting a creature spell in a future set) would also skip the reduction.

**Engine path:**
- engine.rs:2219-2227 (submit_action flashback cost selection)
- engine.rs:1231-1245 (legal_actions flashback cost selection)
- engine.rs:261 (`effective_spell_cost` — the correct reduction function, never called on flashback costs)

**Required check:** 8i

**Affected cards:**
- Heartless Summoning (cost reduction not applied to flashback creature spells)
- Any future ReduceCost effect (same engine path)

