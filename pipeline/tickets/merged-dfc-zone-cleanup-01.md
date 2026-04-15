---
id: merged-dfc-zone-cleanup-01
status: new
card: multiple
created: 2026-04-15T02:39:05Z
kind: consolidated
source_tickets: bloodline_keeper-01, daybreak_ranger-02, delver_of_secrets-01, hanweir_watchkeep-01, instigator_gang-03, kruin_outlaw-02, mayor_of_avabruck-01
---

# DFC name/keywords/subtypes persist after zone change (CR 712.8a)

## Description
When a transformed double-faced card leaves the battlefield, `move_object` (state.rs:572-583) resets `is_transformed` to false but does not reset `obj.name`, `obj.keywords`, or `obj.subtypes`. `apply_transform` (helpers.rs:262-293) wrote back-face values to those fields; they persist into the graveyard, hand, exile, or library. Per CR 712.8a, a DFC outside the battlefield or stack has only the characteristics of its front face.

`has_keyword` and `HasSubtype` checks have registry-based fallbacks that consult `is_transformed` and resolve to the correct face, which masks the keywords/subtypes leak in most paths. `obj_name()` (state.rs:746-748) has no such fallback — it reads `obj.name` directly — so the name leak is directly observable.

Affects any effect that references DFC cards by name, keyword, or subtype in non-battlefield zones (e.g., "return a card named Delver of Secrets from your graveyard").

## Engine path
- state.rs:572-583 (move_object cleanup — does not reset name/keywords/subtypes)
- state.rs:746-748 (obj_name — no registry fallback)
- helpers.rs:262-293 (apply_transform writes back-face fields to object)

## Tests

### test_bloodline_keeper_name_reverts_in_graveyard
Source ticket: bloodline_keeper-01
Implementation: (not yet written)
Scenario: Transform Bloodline Keeper into Lord of Lineage, kill it. Verify the graveyard object's name is "Bloodline Keeper" (front face), not "Lord of Lineage".

### test_daybreak_ranger_subtypes_revert_in_graveyard
Source ticket: daybreak_ranger-02
Implementation: (not yet written)
Scenario: Transform Daybreak Ranger into Nightfall Predator, kill it. Verify the graveyard object's subtypes are ["Human", "Archer"] (front-face), not ["Werewolf"].

### test_delver_keywords_revert_in_graveyard
Source ticket: delver_of_secrets-01
Implementation: (not yet written)
Scenario: Transform Delver into Insectile Aberration, kill it. Verify the graveyard object has no Flying keyword and name "Delver of Secrets".

### test_hanweir_watchkeep_name_reverts_in_graveyard
Source ticket: hanweir_watchkeep-01
Implementation: (not yet written)
Scenario: Transform Hanweir Watchkeep into Bane of Hanweir, kill it. Verify the graveyard object's name is "Hanweir Watchkeep".

### test_instigator_gang_name_reverts_in_graveyard
Source ticket: instigator_gang-03
Implementation: (not yet written)
Scenario: Transform Instigator Gang into Wildblood Pack, kill it. Verify the graveyard object's name is "Instigator Gang".

### test_kruin_outlaw_name_reverts_in_graveyard
Source ticket: kruin_outlaw-02
Implementation: (not yet written)
Scenario: Transform Kruin Outlaw into Terror of Kruin Pass, kill it. Verify the graveyard object's name is "Kruin Outlaw".

### test_mayor_of_avabruck_name_reverts_in_graveyard
Source ticket: mayor_of_avabruck-01
Implementation: (not yet written)
Scenario: Transform Mayor of Avabruck into Howlpack Alpha, kill it. Verify the graveyard object's name is "Mayor of Avabruck".

