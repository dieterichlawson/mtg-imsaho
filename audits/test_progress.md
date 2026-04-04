# Bug Verification Test Progress

Test file: `mtg-engine/tests/audit_bugs.rs`

## VERIFIED (13 bugs with failing tests)

| # | Test name | Bug | Cards affected |
|---|-----------|-----|----------------|
| 1 | `bug_summoning_sickness_not_enforced_for_tap_abilities` | Engine doesn't check summoning_sick for {T} abilities | ~3 cards |
| 2 | `bug_victim_of_night_can_target_vampire_token` | Subtype checks via registry miss tokens | ~18 cards |
| 3 | `bug_etb_trigger_suppressed_when_source_leaves` | Trigger resolution checks zone==Battlefield | ~11 cards |
| 4 | `bug_falkenrath_noble_auto_targets_opponent` | "target player" auto-selects opponent | ~15 cards |
| 5 | `bug_simultaneous_death_triggers_only_fire_once` | Board wipe only triggers death-watch once | ~9 cards |
| 6 | `bug_ghost_quarter_missing_shuffle` | No library shuffle after search | ~4 cards |
| 7 | `bug_ghost_quarter_may_search_is_mandatory` | "may search" auto-searches without choice | ~4 cards |
| 8 | `bug_bonds_of_faith_snapshot_instead_of_continuous` | "as long as" set once at ETB, never re-evaluated | ~4 cards |
| 9 | `bug_planeswalker_damage_uses_damage_marked_not_loyalty` | DealDamage adds damage_marked instead of removing loyalty | ~3 cards |
| 10 | `bug_control_change_not_reverted_at_eot` | "until end of turn" control change never reverted | ~1 card |
| 11 | `bug_spells_cast_this_turn_never_incremented` | Spell cast counter never updated, breaks werewolf transform | all werewolves |
| 12 | `bug_delver_reveal_suppressed_for_non_instant_sorcery` | "you may reveal" only offered for instant/sorcery top cards | 1 card |
| 13 | `bug_once_per_turn_never_clears` | abilities_activated_this_turn persists across turns | ~3 cards |

## FALSE POSITIVE (1)

| Test name | Claimed bug | Actual |
|-----------|-------------|--------|
| `bug_force_attack_ignores_cant_attack` | ForceAttack ignores Pacifism | Engine correctly builds must_attack from eligible list which already excludes Pacified creatures |

## NOT TESTABLE (architectural)

| Category | Count | Reason |
|----------|-------|--------|
| protection targeting | 2 | `can_be_targeted` doesn't take a source parameter — no way to check if source has a protected subtype |
| SBA ordering | 1 | Sequential SBA processing — complex timing, needs engine redesign |

## COSMETIC (not behavioral bugs, but should be fixed)

| Category | Count |
|----------|-------|
| oracle text field mismatch | ~30 |
| log message inaccuracies | ~13 |
| LLM card knowledge | ~5 |

## Summary

- **13 bugs verified** with failing tests, covering **~76 card-level issues**
- **1 false positive** identified and documented
- **2 architectural issues** identified but not testable with simple unit tests
- **~48 cosmetic issues** identified (oracle text, logs, LLM knowledge)
- **~103 remaining uncovered** — mostly instances of the 13 verified bugs or card-specific edge cases
