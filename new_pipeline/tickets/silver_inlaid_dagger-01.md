---
id: silver_inlaid_dagger-01
status: new
card: Silver-Inlaid Dagger
audit_run_id: 2026-04-19-silver_inlaid_dagger-audit
audit_model: sonnet
audit_tokens: 13236
audit_duration: 278
---

## Audit Finding

**Oracle text:**
> As long as equipped creature is a Human, it gets an additional +1/+0.

**Code:**
> if target_obj.subtypes.is_empty() {
                                registry.card_data(target_obj.card_id)
                                    .is_some_and(|d| d.subtypes.iter().any(|s| s == subtype))
                            } else {
                                target_obj.subtypes.iter().any(|s| s == subtype)
                            }

**Description:**
The `check_condition` handler for `EffectCondition::AttachedHasSubtype` (state.rs:1500–1505) uses a two-branch check: if `target_obj.subtypes.is_empty()` it falls back to the registry, otherwise it ONLY checks `target_obj.subtypes`. This diverges from the canonical `CreatureFilter::HasSubtype` pattern (state.rs:869–886), which always checks the registry first and also checks `obj.subtypes`. When a Human creature has a non-Human subtype pushed into `obj.subtypes` at runtime — for example, Olivia Voldaren's {1}{R} ability calls `obj.subtypes.push("Vampire")` — `target_obj.subtypes` becomes `["Vampire"]` (non-empty). The branch then only searches `["Vampire"]`, never consults the registry, and incorrectly returns `false` for `AttachedHasSubtype("Human")`. The result is that Silver-Inlaid Dagger drops its +1/+0 Human bonus the moment any additional subtype is added to the equipped Human creature, even though that creature is still a Human.

**Engine path:** mtg-engine/src/state.rs:1492

**Required check:** 8d

**Affected cards:**
- Butcher's Cleaver
- Bonds of Faith
- Sharpened Pitchfork

## Tests

### human_gains_vampire_subtype_keeps_human_bonus
Scenario: Equip Silver-Inlaid Dagger to a Human creature; activate Olivia Voldaren's {1}{R} ability targeting that creature (adding Vampire to obj.subtypes); the equipped creature should still get +3/+0 total (+2/+0 unconditional + +1/+0 Human bonus) but instead only gets +2/+0.

