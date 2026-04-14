---
id: charmbreaker_devils-01
status: new
card: Charmbreaker Devils
card_file: mtg-engine/src/cards/isd/charmbreaker_devils.rs
created: 2026-04-14T21:23:50Z
audit_run_id: 2026-04-14-charmbreaker_devils-audit
audit_model: opus
audit_tokens: 23951
audit_duration: 476
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.

**Code:**
> `triggers.rs:1306-1307`: `if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield)` gates the entire upkeep trigger resolution on the source being on the battlefield.
> `charmbreaker_devils.rs:50-53`: `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };` independently gates the handler on the source being on the battlefield.

**Description:**
If Charmbreaker Devils' upkeep trigger is placed on the stack and the Devils then leave the battlefield before the trigger resolves (e.g., opponent casts a removal spell in response), the "return an instant or sorcery" ability does not resolve. Per CR 113.7a, an ability on the stack is independent of its source object. Per CR 603.6c, a triggered ability that has been placed on the stack resolves even if the source permanent has since left the battlefield — especially when the ability does not reference the source (it just says "return... from your graveyard to your hand"). Both the engine resolver (triggers.rs:1306) and the card handler (charmbreaker_devils.rs:50) gate on `zone == Battlefield`, so even fixing one without the other would not resolve the issue. The engine gate affects all UpkeepTrigger resolutions, not just this card.

**Engine path:**
- triggers.rs:1306-1311 (resolver battlefield gate)
- charmbreaker_devils.rs:50-53 (handler battlefield gate)

**Required check:** 8b

**Affected cards:**
- Charmbreaker Devils
- All cards with UpkeepTrigger whose abilities don't reference the source (engine-wide resolver gate)

