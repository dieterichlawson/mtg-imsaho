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
