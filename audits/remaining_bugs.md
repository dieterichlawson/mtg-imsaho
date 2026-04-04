# Remaining Bugs — Status

## Test file: mtg-engine/tests/audit_bugs.rs
## Status: 39 tests total, 28 FAIL (verified), 11 PASS (FP/inconclusive)

## Still need tests:
- civilized_scholar: stale attacked_this_turn flag
- creepy_doll: lethal damage + regeneration + coin flip
- boneyard_wurm: view.rs shows base P/T not dynamic (display bug)
- demonmail_hauberk: engine check for sacrifice target availability
- essence_of_the_wild: non-on_resolve path no replacement
- galvanic_juggernaut: force attack ignores can_attack (similar to force-attack FP?)
- grimoire legend rule (DONE - FAILS)
- skirsdag_high_priest: tap two creatures auto-selected
- stitchers_apprentice: trigger_event_index desync
- sturmgeist: draw skipped when leaves battlefield
- undead_alchemist: lifelink interaction (second bug)
