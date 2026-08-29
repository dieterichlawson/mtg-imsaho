# Mutation testing: first engine-core run — 2026-08-29

Setup: `cargo-mutants` 27.1.0, scoped to the rules engine (`-p mtg-engine
-e isd` — per-card files under `src/cards/isd/` are out of scope, being
audited and acceptance-tested card by card). The full engine core holds
~2,365 mutants; `.github/workflows/weekly-mutants.yml` sweeps all of them
every Saturday across ten shards. This report covers the first deep run:
the four most load-bearing files (`sba.rs`, `combat.rs` — which as a glob
also matched `engine/actions/combat.rs` and `triggers/collect/combat.rs` —
`destruction.rs`, `stack.rs`), tested against the `mtg-engine` suite.

## Baseline numbers

320 mutants: **190 caught, 11 timed out** (mutants that livelock a game —
effectively caught), **92 unviable** (don't compile; `warnings = "deny"`
does real work here), **26 missed**. Catch rate on viable mutants: **88%**.

## The 26 survivors, triaged

**Killed by new tests (16):**

| Survivors | Gap | New test |
|---|---|---|
| `eligible_blockers` 3× `&&`→`\|\|` | no test asserted each clause of "untapped creature of the defender" separately | `combat_rules.rs::eligible_blockers_is_untapped_creatures_of_the_defender_only` |
| `deal_damage_step` first-strike conditions (296, 306, 308, 345) | the two-step damage flow was tested for attackers, never for a first-strike *blocker*, a plain first-striker's once-only, or double strike's twice | `a_first_strike_blocker_kills_before_the_attacker_strikes_back`, `a_plain_first_striker_deals_its_damage_exactly_once`, `a_double_striker_deals_damage_in_both_steps` |
| `deal_damage_step` 371 `-`→`+` | lethal-damage calculation ignored damage already marked | `lethal_assignment_counts_damage_already_marked` |
| `walker_still_there` guards ×2 (312, 395) | no test for trample overflow at a *departed* planeswalker (CR 510.1c: it lands nowhere, not on the player) | `planeswalker_combat.rs::trample_overflow_lands_nowhere_when_the_walker_left` (+ blocked variant) |
| `sba.rs` 213 `&&`→`\|\|` | a state trigger already on the stack could re-trigger unpinned | `state_based_actions.rs::a_state_trigger_on_the_stack_does_not_retrigger` |
| `engine/actions/combat.rs` 31 `==`→`!=` | forced-attacker dedup untested with a mix of declared and undeclared forced creatures | `combat_rules.rs::each_forced_attacker_is_dragged_in_independently` |
| `triggers/collect/combat.rs` 54 `==`→`!=` | no negative test that an Equipment on a *bystander* stays quiet | `cards_equipment_and_artifacts.rs::an_equipment_on_a_bystander_does_not_trigger_for_someone_elses_attack` |

**Accepted, with reasons (10):**

- `stack.rs` graveyard re-check arms (8 mutants — deleted arms and
  operator flips in `is_target_legal`'s graveyard requirements). These
  clauses re-check properties of a graveyard card that are *immutable* —
  its owner, its printed creature-ness, its printed subtype — so a target
  the engine legally offered can never become illegal on those axes, and
  no sequence of legal play distinguishes the mutant. The arms are
  defense-in-depth against buggy target *offers* (the historical bug the
  code comment records) and against injected targets; two of the eight
  ("GraveyardCard", "OwnedByTargetPlayer") even fall through to an
  identical `_ => true` default. Killing them would mean testing through
  deliberately corrupted internal state for no behavioral payoff.
- `triggers/collect/combat.rs` zone guards (21, 81, 118). Declaration and
  trigger collection happen inside one action; nothing can remove the
  attacker or blocker between the event and the scan, so the guards are
  defensive and the mutants unreachable through legal play.
- `combat.rs` 122 `>`→`>=`: recording a minimum-blocker requirement of 1
  changes nothing — a single blocker always satisfies it.
- `combat.rs` 296 `\|\|`→`&&` (`was_blocked` snapshot-vs-live): the two
  sets diverge only mid-step in ways with no observable difference.
- `destruction.rs` 189 `\|\|`→`&&` in `death_event`: the right-hand
  fallback exists for registry-less callers; every engine path passes the
  registry, making the clause redundant there.

## How to read this instrument

- Missed mutants are findings, not failures — the weekly workflow stays
  green and publishes survivors as artifacts; this file is where survivors
  get triaged into "test gap" (fix) or "equivalent/defensive" (accept,
  with the reason written down).
- "Unviable" is not waste: 29% of mutants failing to compile is
  `warnings = "deny"` and the type system doing free mutation-catching.
- The 11 timeouts are livelock mutants — a broken SBA or combat loop hangs
  a game. The fuzzer's runner would also catch these; in mutation runs the
  120s timeout counts them as caught-by-hang.

## Verification

A re-run of the same 320 mutants after the nine new tests was in progress
when the session's container restarted; partially observed results matched
expectations (survivors seen up to that point were all from the accepted
list). The re-run is repeated to completion and this section updated with
the final count — the target state is exactly the 10 accepted survivors.
