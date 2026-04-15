---
id: nephalia_drownyard-01
status: deduped
card: Nephalia Drownyard
card_file: mtg-engine/src/cards/isd/nephalia_drownyard.rs
created: 2026-04-14T21:30:20Z
audit_run_id: 2026-04-14-nephalia_drownyard-audit
audit_model: opus
audit_tokens: 5372
audit_duration: 126
deduped_into: merged-activated-no-stack-01
---

## Audit Finding

**Oracle text:**
> {1}{U}{B}, {T}: Target player mills three cards.

**Code:**
> `Action::ActivateAbility` handler at engine.rs:2559 calls `behavior.on_activate_ability()` directly at engine.rs:2719, resolving the mill effect immediately without placing the ability on the stack.

**Description:**
Per CR 602.2a, when a player activates an activated ability, the ability is placed on the stack. It becomes the topmost object on the stack, and opponents receive priority to respond (e.g., with Stifle, Trickbind, or other counterspells that target activated abilities). The engine bypasses this entirely — the `ActivateAbility` action handler pays costs and then immediately calls `on_activate_ability`, which executes `mill_cards` in the same action resolution. The opponent never gets a chance to respond to the mill ability. This is a known engine-wide architectural limitation affecting all non-mana activated abilities.

**Engine path:**
- engine.rs:2559 (ActivateAbility handler entry)
- engine.rs:2717-2719 (non-X ability: `on_activate_ability` called immediately)
- engine.rs:2711 (X-ability path also calls `on_activate_ability` immediately after funding)
- cards/isd/nephalia_drownyard.rs:65-69 (`on_activate_ability` calls `mill_cards`)

**Required check:** 8i

**Affected cards:**
- Nephalia Drownyard
- All cards with non-mana activated abilities (engine-wide)
