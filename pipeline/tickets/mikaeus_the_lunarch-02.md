---
id: mikaeus_the_lunarch-02
status: closed-duplicate
card: Mikaeus, the Lunarch
card_file: mtg-engine/src/cards/isd/mikaeus_the_lunarch.rs
created: 2026-04-14T20:45:48Z
audit_run_id: 2026-04-14-mikaeus_the_lunarch-audit
audit_model: opus
audit_tokens: 19324
audit_duration: 400
duplicate_of: merged-activation-cost-variants-01
---

## Audit Finding

**Oracle text:**
> {T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.

**Code:**
> `if let Some(obj) = state.get_object_mut(object_id) {`
> `    let count = obj.counters.entry(CounterType::PlusOnePlusOne).or_insert(0);`
> `    if *count > 0 { *count -= 1; }`
> `}`
> — mikaeus_the_lunarch.rs:105-110 (inside `on_activate_ability`)

**Description:**
"Remove a +1/+1 counter from Mikaeus" is an activation COST (CR 602.2b), not an effect. Per CR 602.2, costs (mana, tap, sacrifice, counter removal) are paid at 602.2e-f before the ability goes on the stack and resolves. The engine natively handles tap costs (engine.rs:2652-2654), mana costs (engine.rs:2643-2647), and sacrifice costs (engine.rs:2660-2670) in the formal cost-payment sequence before calling `on_activate_ability`. However, `ActivatedAbilityDef` has no field for counter-removal costs, so the card implements it manually inside `on_activate_ability` — interleaving cost payment with effect execution. Since the engine resolves activated abilities immediately (without putting them on the stack), this produces correct behavior today. But the counter removal lacks formal cost semantics: it does not fire counter-removal events, and if the engine added ability-stack support (allowing Stifle-type effects), the counter would not have been removed as a cost before the ability went on the stack. This is a known engine limitation (documented in auditor-insights.md under "Counter-removal activation costs are not supported by the engine").

**Engine path:**
- engine.rs:2652-2654 (tap cost paid natively)
- engine.rs:2717-2719 (non-X `on_activate_ability` called — counter removal happens here)
- mikaeus_the_lunarch.rs:105-110 (manual counter removal)
- mikaeus_the_lunarch.rs:112-119 (effect execution in same handler)

**Required check:** 8c / 8i

**Affected cards:**
- Mikaeus, the Lunarch
- Any card with counter-removal activation costs (see insight "Counter-removal activation costs are not supported by the engine")
