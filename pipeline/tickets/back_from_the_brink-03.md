---
id: back_from_the_brink-03
status: closed-duplicate
card: Back from the Brink
card_file: mtg-engine/src/cards/isd/back_from_the_brink.rs
created: 2026-04-14T21:24:13Z
audit_run_id: 2026-04-14-back_from_the_brink-audit
audit_model: opus
audit_tokens: 20377
audit_duration: 499
duplicate_of: merged-activation-cost-variants-01
---

## Audit Finding

**Oracle text:**
> Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card.

**Code:**
> back_from_the_brink.rs:105: `state.move_object(creature_id, Zone::Exile, registry);` — exile happens inside `on_activate_ability()`, which is the resolution phase, not the cost-payment phase.

**Description:**
In the oracle text, the colon separates cost from effect. Everything before the colon ("Exile a creature card from your graveyard and pay its mana cost") is cost; everything after ("Create a token that's a copy of that card") is effect. Per CR 602.2b, costs are paid during activation (before the ability is placed on the stack), not during resolution. The engine's `ActivatedAbilityDef` supports mana costs, tap costs, and sacrifice costs, but has no field for exile-from-graveyard costs. As a result, the card must handle the exile in `on_activate_ability()`, deferring it to the resolution phase. Currently masked by Finding 2 (no stack makes everything atomic), but if the engine adds stack support for activated abilities, the exile must move to cost payment. Additionally, if ability-countering effects (like Stifle) are implemented, the current code would incorrectly skip the exile when the ability is countered — costs should be paid regardless of whether the ability resolves.

**Engine path:**
- back_from_the_brink.rs:85-112 (on_activate_ability — exile at line 105)
- cards/mod.rs (ActivatedAbilityDef struct — no exile cost field)

**Required check:** 8i

**Affected cards:**
- Back from the Brink
- Any future card with exile-from-zone as an activation cost
