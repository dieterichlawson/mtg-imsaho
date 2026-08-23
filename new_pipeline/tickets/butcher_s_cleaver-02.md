---
id: butcher_s_cleaver-02
status: new
card: Butcher's Cleaver
audit_run_id: 2026-04-19-butcher_s_cleaver-audit
audit_model: sonnet
audit_tokens: 32807
audit_duration: 713
---

## Audit Finding

**Oracle text:**
> As long as equipped creature is a Human, it has lifelink.

**Code:**
> if target_obj.subtypes.is_empty() {
    registry.card_data(target_obj.card_id)
        .is_some_and(|d| d.subtypes.iter().any(|s| s == subtype))
} else {
    target_obj.subtypes.iter().any(|s| s == subtype)
}

**Description:**
The `AttachedHasSubtype` branch in `check_condition` (state.rs) treats `obj.subtypes` as the complete, authoritative subtype set whenever it is non-empty. For non-token, non-DFC cards, `obj.subtypes` starts as an empty Vec and only receives runtime pushes — native subtypes always live solely in the registry. Two cards in the set push subtypes at runtime: Olivia Voldaren pushes "Vampire" to any creature she damages, and Grimoire of the Dead pushes "Zombie" to every creature it reanimates. After either effect fires on a Human creature, `obj.subtypes` becomes non-empty (e.g. `["Vampire"]`) but does not include "Human". The `AttachedHasSubtype("Human")` condition then checks only `obj.subtypes`, finds no "Human", and returns false — incorrectly stripping lifelink from a creature that is still a Human per its oracle text (both Olivia and Grimoire add types 'in addition to other types'). The canonical `matches_filter::HasSubtype` pattern (state.rs:881) avoids this by always checking the registry first and treating `obj.subtypes` as an additive fallback; `AttachedHasSubtype` must be aligned to that pattern. The same defect affects `AttachedLacksSubtype`, which delegates to `AttachedHasSubtype` and therefore inverts the wrong result: for Bonds of Faith, a Human-turned-Vampire would incorrectly lose the +2/+2 bonus AND incorrectly gain the 'can't attack or block' restriction simultaneously.

**Engine path:** mtg-engine/src/state.rs:1500

**Required check:** 8d

**Affected cards:**
- Bonds of Faith
- Sharpened Pitchfork
- Silver Inlaid Dagger

## Tests

### lifelink_lost_after_olivia_adds_vampire_subtype
Scenario: A Human creature is equipped with Butcher's Cleaver; Olivia Voldaren's ability deals 1 damage to that creature, pushing "Vampire" into obj.subtypes; on the equipped creature's next attack, it should have lifelink (it is still a Human), but AttachedHasSubtype("Human") incorrectly returns false because obj.subtypes is ["Vampire"] and the registry is not consulted.

### lifelink_lost_after_grimoire_zombie_reanimate
Scenario: A Human creature dies, goes to the graveyard with obj.subtypes empty, then Grimoire of the Dead reanimates it — pushing "Zombie" into obj.subtypes; when later equipped with Butcher's Cleaver, the creature should have lifelink (still a Human per oracle), but AttachedHasSubtype("Human") returns false because obj.subtypes is ["Zombie"].

