---
id: gavony_township-01
status: fixed
card: Gavony Township
audit_run_id: 2026-04-19-gavony_township-audit
audit_model: sonnet
audit_tokens: 33965
audit_duration: 2110
fixed_sha: 74c31e5
fixed_at: 2026-08-24T00:53:36Z
test_file: mtg-engine/tests/tap_cost_legality.rs
fix_note: CR 602.2h: a permanent paying an ability's {T} cost is excluded from the autotap mana pool for that same ability.
---

## Audit Finding

**Oracle text:**
> {2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.

**Code:**
> let ability_sources: Vec<_> = if ability_has_sac_this {
    early_mana_sources.iter()
        .filter(|s| s.object_id != obj_id)
        .cloned()
        .collect()
} else {
    early_mana_sources.clone()
};

**Description:**
The auto-tap plan for Gavony Township's activated ability can include the Township itself as a mana source via its {T}: Add {C} mana ability, while simultaneously requiring the Township to be tapped for the {T} in the activation cost. The source-exclusion guard at engine.rs:722–729 only fires when the ability has SacrificeCost::SacrificeThis; for SacrificeCost::None with requires_tap: true, the source is included in ability_sources via the else branch at line 728. This allows compute_autotap (mana.rs Phase 3, lines 303–341) to select Township as a generic-mana source when other sources are exhausted by the colored requirements. submit_action then taps Township for mana (producing {C}), and subsequently sets tapped = true again for the activation cost — a no-op, since it is already tapped. The net effect: the engine offers the ability as legal when the player has only 3 other untapped mana sources (e.g., 2 Forests + 1 Plains) instead of the legally required 4, by crediting the Township's own {T}: Add {C} mana production toward its {2}{G}{W}, {T} cost. A single tap is used for two purposes, which is illegal per CR 602.2h. All five ISD utility lands share this pattern and are affected.

**Engine path:** mtg-engine/src/engine.rs:722

**Required check:** 8c

**Affected cards:**
- Kessig Wolf Run
- Moorland Haunt
- Nephalia Drownyard
- Stensia Bloodhall

## Tests

### gavony_township_self_not_in_mana_plan
Scenario: With exactly 2 Forests, 1 Plains, and Gavony Township on the battlefield and no mana in pool, the {2}{G}{W},{T} ability should not appear as a legal action (only 3 non-Township mana available, need 4), but the engine incorrectly offers it by including Township in its own auto-tap mana plan.

