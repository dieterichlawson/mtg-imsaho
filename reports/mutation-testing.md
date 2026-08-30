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

**Killed by new tests:**

| Survivors | Gap | New test |
|---|---|---|
| `eligible_blockers` 3× `&&`→`\|\|` | no test asserted each clause of "untapped creature of the defender" separately | `combat_rules.rs::eligible_blockers_is_untapped_creatures_of_the_defender_only` |
| `deal_damage_step` 306 first-strike gate | the two-step damage flow was tested for attackers, never for a plain first-striker's once-only or double strike's twice | `a_plain_first_striker_deals_its_damage_exactly_once`, `a_double_striker_deals_damage_in_both_steps` |
| `deal_damage_step` 345 blocker first-strike gate | the first-strike-blocker test passed *vacuously* under this mutant — the dead blocker's `damage_marked` clears on the zone change, so asserting it zero proved nothing (caught by the verification run below) | `a_first_strike_blocker_kills_before_the_attacker_strikes_back`, strengthened to assert the blocker survived |
| `deal_damage_step` 308 blocked-ness check | nothing pinned CR 509.2's "blocked forever": a blocker *removed from combat* (regeneration, control change) empties the assignment while the attacker stays blocked; a blocker that merely dies stays in the snapshot, so the obvious repro never reaches this branch (caught by the verification run below) | `combat_rules.rs::a_blocked_attacker_whose_blocker_left_combat_hits_nobody` |
| `deal_damage_step` 371 `-`→`+` | lethal-damage calculation ignored damage already marked | `lethal_assignment_counts_damage_already_marked` |
| `walker_still_there` guards ×2 (312, 395) | no test for trample overflow at a *departed* planeswalker (CR 510.1c: it lands nowhere, not on the player) | `planeswalker_combat.rs::trample_overflow_lands_nowhere_when_the_walker_left` (+ blocked variant) |
| `sba.rs` 213 `&&`→`\|\|` | a state trigger already on the stack could re-trigger unpinned | `state_based_actions.rs::a_state_trigger_on_the_stack_does_not_retrigger` |
| `engine/actions/combat.rs` 31 `==`→`!=` | the walker-attack dedup ("drop an entry whose attacker already attacks the player") only diverges when one declaration mixes attacks on the player and on a walker; the forced-attacker test never touched this filter (caught by the verification run below) | `planeswalker_combat.rs::attacking_a_walker_alongside_attacks_on_the_player` |
| `triggers/collect/combat.rs` 54 `==`→`!=` | the equipment-bystander test asserts a *negative*, which an inverted zone filter also satisfies — and no Innistrad card declares `AnyCreatureAttacks`, so the watcher scan had no positive coverage at all (caught by the verification run below) | `trigger_dispatch.rs::an_attack_watcher_hears_the_attack_from_the_battlefield_only` (registers a test-only watcher card) |

**Accepted, with reasons (16 mutants, 13 normalized lines in `reports/mutants-accepted.txt`):**

- `stack.rs` graveyard re-check arms (10 mutants — deleted arms and
  operator flips in `is_target_legal`'s graveyard requirements, including
  the zone-table `==`→`!=` at 67, whose in-code comment already documents
  the masking). These
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

Re-running the same 320 mutants after the nine new tests: **203 caught,
12 timed out, 85 unviable, 19 missed** (319 recorded — the final mutant's
log write hit a full disk; the unrecorded one, `stack.rs:67` `==`→`!=`,
is in the accepted graveyard-arm family above). Some baseline "unviable"
outcomes shifted to "caught" between runs — build-order sensitivity in
cargo-mutants, not a suite change.

The instrument audited itself: four of the baseline's claimed kills were
false, and the 19 survivors exposed them. Two tests passed vacuously under
their mutants (345: cleared `damage_marked` after a zone change; 54: a
negative assertion an inverted filter also satisfies), one targeted the
wrong condition (31), and one gap needed a path no test drove (308:
removal from combat vs. death). All four are re-fixed with tests verified
to FAIL under manual application of their exact mutants — a check worth
keeping: a mutation-motivated test isn't done until it has been watched
killing its mutant.

Final state: the 15 remaining recorded survivors (plus the one unrecorded)
are exactly the accepted list. One caveat on the accepted-list format: the
workflow strips line:col, so an accepted line like "`\|\|`→`&&` in
deal_damage_step" masks *any* such mutant in that function (296 accepted,
but a regression re-surfacing 345 would be masked too). The price of
edit-stable comparisons; the per-line reasons above are the record.

# The full engine-core sweep — 2026-08-30

The first scheduled weekly run (all 2,367 engine-core mutants, run
33255670070) filed nine per-shard survivor issues (#26–#34). A local
full sweep against the same-day HEAD gave the exact picture: **1,125
caught, 32 timed out** (livelocks the suite catches by hanging), **789
unviable, 421 missed** — 241 unique survivors after normalization, 12 of
them already on the accepted list.

## Dispositions

Three buckets now exist, and every survivor is in exactly one:

1. **Killed** — a test written for it, watched failing under the exact
   mutant before it counts. This round: the composite branches of
   submitted-target validation, the CardBehavior hook defaults and the
   DFC name fallback, the auto-tap planner contract, X-funding
   arithmetic and bounds, the targeted-pump accumulation, same-controller
   control changes, the stack-entry accessors, printed colors
   (CR 202.2/204.2) — and the whole `invariant_checker.rs` battery: the
   fuzzing oracle's ~24 invariant families each verified to flag their
   corruption, with a rich clean state (populated libraries, graveyards,
   stack, attachments, loyalty, combat) pinning the false-positive
   direction that corruption tests alone cannot see.
2. **Accepted** (`reports/mutants-accepted.txt`) — judged equivalent or
   out of scope, each under a written reason: RNG mixing internals
   (determinism is the contract, pinned by the replay check), a
   runner-facing helper no engine path calls, an effect variant nothing
   emits yet, display/log-text arms, an identity mutant, re-lookup of a
   failed key, and per-card files outside the engine core.
3. **Backlog** (`reports/mutants-backlog.txt`) — genuine gaps, kept
   visible and worked down by the daily fixer, but not re-filed by the
   weekly workflow. Deleting a line is the "killed" ceremony; moving one
   to accepted needs a reason here.

The weekly workflow now files an issue only for survivors in none of the
three buckets — i.e. *new* regressions — and its cargo-mutants version is
pinned (27.1.0), because mutant names render differently across versions
and the suppression lists match on the rendered name.
