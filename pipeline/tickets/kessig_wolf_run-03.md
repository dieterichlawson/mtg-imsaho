---
id: kessig_wolf_run-03
status: closed-duplicate
card: Kessig Wolf Run
card_file: mtg-engine/src/cards/isd/kessig_wolf_run.rs
created: 2026-04-14T21:29:34Z
audit_run_id: 2026-04-14-kessig_wolf_run-audit
audit_model: opus
audit_tokens: 14580
audit_duration: 309
duplicate_of: merged-activated-no-stack-01
---

## Audit Finding

**Oracle text:**
> {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.

**Code:**
> `StackEntry` enum has only `Spell(ObjectId)` and `Trigger(PendingTrigger)` variants — state.rs:10-15. No `Ability` variant exists.
> `Action::ActivateAbility` handler (engine.rs:2633-2724) calls `on_activate_ability` directly without pushing to the stack.

**Description:**
Per CR 602.2a, activating an activated ability puts it on the stack. It remains on the stack until it resolves, is countered, or otherwise leaves. Players receive priority after an ability is put on the stack (CR 117.3b) and may respond with instants or other abilities before it resolves. The engine has no `StackEntry` variant for activated abilities — `on_activate_ability` fires the effect immediately at activation time. This means opponents cannot respond to Kessig Wolf Run's pump ability: they cannot remove the target creature, counter the ability (e.g., Stifle), or take any action between the ability's activation and its effect. For the X-cost path, the ChooseXFunding prompt (engine.rs:2679-2706) creates a window where costs are partially paid but no stack entry exists, and the effect fires immediately after funding.

**Engine path:**
- state.rs:10-15 (StackEntry enum — no Ability variant)
- engine.rs:2633-2724 (ActivateAbility handler — no stack push)
- engine.rs:3199-3214 (ChooseXFunding handler — fires on_activate_ability directly)

**Required check:** 8i

**Affected cards:**
- Kessig Wolf Run
- Every card with a non-mana activated ability
