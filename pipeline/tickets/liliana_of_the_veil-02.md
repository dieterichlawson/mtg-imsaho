---
id: liliana_of_the_veil-02
status: closed-duplicate
card: Liliana of the Veil
card_file: mtg-engine/src/cards/isd/liliana_of_the_veil.rs
created: 2026-04-14T20:55:10Z
audit_run_id: 2026-04-14-liliana_of_the_veil-audit
audit_model: opus
audit_tokens: 11904
audit_duration: 290
duplicate_of: merged-temp-effect-zone-persist-02
---

## Audit Finding

**Oracle text:**
> (All three loyalty abilities)

**Code:**
> In `state.rs:572-583`, the battlefield-exit cleanup block does NOT clear `abilities_activated_this_turn`. In `state.rs:592-596`, the battlefield-entry block clears `card_state` and sets `summoning_sick` but does NOT clear `abilities_activated_this_turn`. The sentinel value 999 (engine.rs:2972) persists through zone changes.

**Description:**
Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. If Liliana activates a loyalty ability (setting the sentinel 999 in `abilities_activated_this_turn`), then is bounced to hand and recast in the same turn, the sentinel persists because neither the battlefield-exit cleanup (state.rs:572-583) nor the battlefield-entry cleanup (state.rs:592-596) clears `abilities_activated_this_turn`. The guard at engine.rs:871 (`already_used = obj.abilities_activated_this_turn.contains(&999)`) then incorrectly prevents the re-entered planeswalker from activating a loyalty ability. Per CR 606.3, the restriction is on the permanent, and since the re-entered permanent is a new object per CR 400.7, it should be allowed to activate a loyalty ability.

**Engine path:**
- state.rs:572-583 (battlefield-exit cleanup — missing `abilities_activated_this_turn.clear()`)
- state.rs:592-596 (battlefield-entry cleanup — missing `abilities_activated_this_turn.clear()`)
- engine.rs:871 (guard that checks the sentinel)
- engine.rs:2972 (sentinel set on loyalty activation)

**Required check:** 8c

**Affected cards:**
- Liliana of the Veil
- All planeswalkers (engine-wide bug)
- All permanents with once-per-turn activated abilities
