---
id: back_from_the_brink-02
status: new
card: Back from the Brink
audit_run_id: 2026-04-19-back_from_the_brink-audit
audit_model: sonnet
audit_tokens: 35564
audit_duration: 870
---

## Audit Finding

**Oracle text:**
> The token will be a copy of the front face and it won't be able to transform.

**Code:**
> for &token_id in &all_ids {
    if let Some(obj) = self.get_object_mut(token_id) {
        obj.card_id = card_id;
        obj.is_legendary = is_legendary;
    }
}

**Description:**
`create_token_copy` (state.rs:534-539) stamps every created token — including the Back from the Brink token — with the source creature's `card_id`. For DFC creatures (e.g., Reckless Waif, card_id carries `TriggerKind::Upkeep` in its `triggered_abilities`), the token now shares the DFC's full `CardBehavior`. At each upkeep, `triggers.rs:837-857` scans every battlefield object; for the token, `face_trigger_description(registry, card_id, Upkeep, is_transformed=false)` reads the DFC's front-face `triggered_abilities` and returns a non-empty description, so an `UpkeepTrigger` is created with the token's `object_id`. When resolved, `behavior.on_upkeep(state, token_id, …)` is called, which invokes the werewolf transform logic (`should_transform`, then `helpers::apply_transform`). Neither `should_transform` nor `apply_transform` has an `is_token` guard, so the token transforms into the back face if the condition is met (e.g., zero spells cast last turn). The ruling explicitly states these tokens cannot transform.

**Engine path:** mtg-engine/src/state.rs:534

**Required check:** 8j

**Affected cards:**
- Cackling Counterpart

## Tests

### dfc_token_cannot_transform_on_upkeep
Scenario: A Reckless Waif token created by Back from the Brink should not transform at the start of upkeep even when no spells were cast last turn.

