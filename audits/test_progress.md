# Bug Verification Test Progress

Test file: `mtg-engine/tests/audit_bugs.rs`

## VERIFIED (7 bugs with failing tests)

| # | Test name | Bug | Cards affected |
|---|-----------|-----|----------------|
| 1 | `bug_summoning_sickness_not_enforced_for_tap_abilities` | Engine doesn't check summoning_sick for {T} abilities | 3 cards (Avacynian Priest, Mikaeus, Furor of the Bitten) |
| 2 | `bug_victim_of_night_can_target_vampire_token` | Subtype checks via registry miss tokens | 18 cards |
| 3 | `bug_etb_trigger_suppressed_when_source_leaves` | Trigger resolution checks zone==Battlefield | 11 cards |
| 4 | `bug_falkenrath_noble_auto_targets_opponent` | "target player" auto-selects opponent | 15 cards |
| 5 | `bug_simultaneous_death_triggers_only_fire_once` | Board wipe only triggers death-watch once | 9 cards |
| 6 | `bug_ghost_quarter_missing_shuffle` | No library shuffle after search | 4 cards |
| 7 | `bug_ghost_quarter_may_search_is_mandatory` | "may search" auto-searches without choice | 4 cards |

## FALSE POSITIVE (1)

| Test name | Claimed bug | Actual |
|-----------|-------------|--------|
| `bug_force_attack_ignores_cant_attack` | ForceAttack ignores Pacifism | Engine correctly builds must_attack from eligible list which already excludes Pacified creatures |

## SKIPPED (not testable / cosmetic)

| Category | Count | Reason |
|----------|-------|--------|
| oracle text mismatch | 5 | Cosmetic display string |
| log message | 13 | Cosmetic log text |
| LLM knowledge | 2 | AI player guidance |
| test enshrines wrong behavior | 4 | Meta-issue about existing tests |
| protection targeting | 2 | Narrow interaction, primarily affects combat which IS implemented |
| "as long as" snapshot | 4 | Test setup too complex (Bonds of Faith ETB + transform), bug is real but hard to test in isolation |
| engine: planeswalker damage | 3 | Need planeswalker card setup, TODO for future |

## Summary

- 7 bugs verified with failing tests covering ~64 issues
- 1 false positive identified
- ~24 cosmetic/skipped issues
- ~175 NEEDS_REVIEW issues (many are duplicates or instances of the 7 verified bugs)
