---
id: cackling_counterpart-02
status: new
card: Cackling Counterpart
audit_run_id: 2026-04-18-cackling_counterpart-audit
audit_model: sonnet
audit_tokens: 15636
audit_duration: 272
---

## Audit Finding

**Oracle text:**
> Any "enters" triggered ability of the copied creature will trigger when the token enters the battlefield. Any "as [this creature] enters" or "[this creature] enters with" abilities of the chosen creature will also work.

**Code:**
> let token_id = state.create_token_copy(*target_id, controller, registry);
let name = state.get_object(token_id).map(|o| o.name.clone()).unwrap_or_default();
state.log(crate::state::LogLevel::Event,
    format!("Cackling Counterpart creates a token copy of {name}"));

**Description:**
The ruling specifies that ETB triggered abilities of the copied creature fire when the token enters. The engine's `create_token_internal` (state.rs:484) emits an `EnteredBattlefield` event for every token, and `create_token_copy` patches `card_id` for all tokens before any trigger dispatch occurs, so the trigger system should see the correct `CardBehavior` and fire ETB triggers. The code looks correct, but no test in `mtg-engine/tests/` exercises this path for Cackling Counterpart — specifically, no test copies a creature with a non-trivial ETB triggered ability and verifies the trigger fires on the token.

**Required check:** 8j

## Tests

### cackling_counterpart_token_copy_etb_triggers_fire
Scenario: Cackling Counterpart copies a creature that has an ETB triggered ability; after resolution the trigger should be on the stack for the token, not absent.

