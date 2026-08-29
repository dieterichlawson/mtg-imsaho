# Invariant fuzzing: first campaign findings — 2026-08-29

Context: after the per-card audits converged, the open question was how to
measure progress with an instrument whose errors are not correlated with an
LLM auditor's. The answer built here is seeded random-game fuzzing under
two oracles that involve no judgment about what any card does:

- **State invariants** (`mtg_engine::invariants`): structural properties of
  `GameState` checked at every decision point (`check_core` even
  mid-resolution; `check_settled` adds what CR 704.3 guarantees right
  before priority — tokens ceased, dead creatures dead, losses applied,
  auras attached, library order and zone in agreement).
- **Replay determinism**: the same seed must replay the same game,
  log-line for log-line. A divergence means a `HashMap`/`HashSet`'s
  per-process order leaked into the game.

Entry points: `scripts/fuzz.sh` (campaign), `mtg-runner --seed N
--check-invariants` (one replayable game), and
`mtg-player/tests/fuzz_random_games.rs` (in-suite battery so `cargo test`
guards both oracles forever).

## Bugs found (all fixed, same day)

1. **Mana pools iterated in hash order** — `ManaPool`/`FundingOptions`
   kept mana in a `HashMap`; display, funding clones, and drains iterated
   it, so one seed's two runs showed `pool: Black:1 Blue:1` vs
   `pool: Blue:1 Black:1` and drifted apart. Now `BTreeMap` (canonical
   WUBRG+C order). Commit 8955218.
2. **Combat state iterated in hash order** — `CombatState`'s attackers,
   blocker assignments, and first-strike bookkeeping were `HashMap`/
   `HashSet`; the damage step dealt damage, ordered events, and built the
   `ChooseBlockers` prompt in per-process order. Now `BTreeMap`/`BTreeSet`
   in `ObjectId` order. Commit d77f8cc.
3. **Legend-rule SBA livelock** — with two same-name legends under one
   player (two Grimgrins, seed 74, rb-vampires vs ub-zombies), the legend
   rule raised its keep-choice, reported an action taken, and was
   re-checked by the engine's SBA loop before anyone could answer —
   re-raising the same prompt forever, freezing the game inside turn 18
   with no decision points. The keep-choice now holds while another prompt
   is pending; every other state-based action still runs in that window (a
   blanket "no SBAs while awaiting" guard broke concede-during-combat and
   was rejected by the suite). Commit 2b9e031, regression-tested and
   mutation-checked.

Also fixed as fallout: the runner serialized the full game state (log
included) to the hot-reload save on every action, which made long AI-vs-AI
games quadratic and indistinguishable from hangs; saves are now written
only for interactive games (commit 860faba).

## Campaign result

After the fixes: **1000 seeded games (100 per deck pairing across
gw-humans / rb-vampires / ub-zombies / ug-spider-spawning, mirrors
included), 0 invariant violations, 0 non-terminations, and replay
determinism spot checks clean.** Full workspace suite: 1629 passing, zero
warnings.

## How to read this

Three engine bugs — two determinism, one livelock — survived 249 per-card
audits, ~1626 tests, and a manual read of every audit file, and fell to the
fuzzer within its first hour. That is the expected shape: the audits
compare card code to oracle text and are blind to engine-level emergent
behavior; the fuzzer is blind to card semantics but sees exactly that
behavior. The two instruments are complementary, and neither's clean bill
implies the other's.

Standing guardrails now in place: the in-suite fuzz battery runs with every
`cargo test`; `scripts/fuzz.sh N` scales to arbitrary campaign sizes with
replayable failures. The companion measurement plan for the LLM audits
themselves (seeded-bug sensitivity, false-positive rate, capture–recapture
remaining-bug estimation) is `docs/plans/audit-calibration.md` with its
harness in `scripts/calibration/seed_bugs.sh`.

# Second wave: coverage decks — 2026-08-29

The four curated decks reached ~60 cards; `decks/coverage/` (ten two-color
decks, every castable card exactly once, pinned by `deck_coverage.rs`)
put the whole pool in reach, and `scripts/fuzz.sh` now defaults to it.
The first 550-game pilot produced ten failures with three signatures, and
the widened invariant battery (attachment-graph shape, combat/step
coherence, controller checks, trigger-queue emptiness at priority,
CR 400.7 reset completeness) found two more classes on top:

4. **Equip resolved for a dead Equipment** — Blazing Torch destroyed in
   response to its own equip ability still "attached" from the graveyard;
   `resolve_equip` re-checked the creature but never the Equipment
   (CR 701.3c). Commit 66386ba.
5. **Removing a creature from combat left residue** — regeneration's
   `remove_from_combat` cleared `attackers` and `blocker_assignments` but
   not `blocked_attackers` / `planeswalker_defenders` /
   `dealt_first_strike`. Commit 725ff16.
6. **Evil Twin's copy-entry guard could leak** — a copy choice whose
   chosen creature ceased to exist returned without disarming the SBA
   exemption, leaving an unkillable 0/0. The invariant now honors the
   entry window (armed + summoning-sick) and flags a guard that outlives
   it. Commit 998452e.
7. **A stolen creature stayed in combat** — CR 506.4d was unimplemented;
   `change_control` now removes the creature from combat, which is what
   makes "an attacker is controlled by the active player" a true
   invariant. Commit 336a1ce.
8. **Tapping reached cards in hand** — Claustrophobia's enters-tap,
   resolving through last-known information after Lantern Spirit bounced
   itself, tapped the card in its owner's hand; `GameState::tap` now
   refuses anything not on the battlefield (CR 110.5). Commit fb58f6d.
9. **Steps advanced over a non-empty stack** — the game loop's fallback
   paths advanced steps unconditionally, so a trigger from attacker
   declaration could resolve a full turn late (Geist of Saint Traft's
   Angel created in the NEXT turn's combat, caught by the
   attacker-controller invariant at seed 290). Every fallback now
   resolves the stack first (CR 500.2). Commit 78ce575.

The widened battery then caught one more flow bug the narrower one could
not see:

10. **Structured prompts never reached the player** — X-funding and
    exile-from-graveyard cost prompts enumerate no flat actions, and the
    loop's "no legal actions" fallback skipped the callback for them: the
    cast stranded mid-payment and the standing unanswered choice made
    every later spell's cleanup defer forever, piling resolved cards up
    orphaned in the stack zone (stack-accounting invariant, seed 550,
    Corpse Lunge). Commit 7a6a652.

After the fixes: the pilot's failing seeds all pass, the in-suite battery
runs clean (now including Corpse Lunge for the structured-prompt path),
a 5,500-game coverage campaign reports zero failures, and the full
workspace suite stands at 1,636 passing with zero warnings.

The pattern of this wave is worth naming: the four curated decks had
never exercised planeswalker combat prompts, copy-entry choices,
structured cost prompts, or mid-combat control changes, so the game loop's
handling of those was simply untested at the system level. Full-pool
coverage decks turned "every card audited" into "every card actually
played under oracle supervision" — and all seven bugs this wave were in
the engine, not in any card's text: exactly the class per-card audits are
structurally blind to. Not one fix touched a card file.
