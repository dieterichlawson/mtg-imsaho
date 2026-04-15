---
id: back_from_the_brink-02
status: deduped
card: Back from the Brink
card_file: mtg-engine/src/cards/isd/back_from_the_brink.rs
created: 2026-04-14T21:24:13Z
audit_run_id: 2026-04-14-back_from_the_brink-audit
audit_model: opus
audit_tokens: 20377
audit_duration: 499
deduped_into: merged-activated-no-stack-01
---

## Audit Finding

**Oracle text:**
> Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.

**Code:**
> engine.rs:2716-2723: non-X activated abilities call `behavior.on_activate_ability()` immediately after cost payment with no stack involvement. For X-cost abilities, the deferred path (engine.rs:2679-2715) similarly calls `on_activate_ability` directly after X funding, never placing the ability on the stack.

**Description:**
Per CR 602.2a, activating an activated ability creates an instance of it on the stack. Players receive priority after the ability is placed on the stack, allowing opponents to respond (e.g., exiling creatures from the controller's graveyard in response, removing Back from the Brink, or casting Stifle to counter the ability). In the engine, activated abilities resolve immediately — `on_activate_ability` is called directly after cost payment with no stack entry and no priority pass. This means the opponent can never respond to Back from the Brink's activation. This is an engine-wide issue affecting all cards with activated abilities, not specific to this card.

**Engine path:**
- engine.rs:2559-2725 (ActivateAbility handler — entire flow is atomic)

**Required check:** 8c

**Affected cards:**
- Back from the Brink
- All cards with activated abilities (engine-wide)
