# Draft Elo Experiment — Plan (v2)

## Goal

Estimate **Draft Elo** for each of four thinking levels — `minimal`, `low`, `medium`, `high` — where Draft Elo measures the expected performance of a deck produced by L-level drafting+building, played by a fixed reference piloter against opponents drafted by other levels. Drafting and deck-building are intentionally combined into one number; we are not separating them.

The model under test is `gemini-3.1-flash-lite-preview`. Format is Innistrad (`isd`) booster draft.

This document covers **Phase 0 only** — the initial pilot. Whether and how to expand will be decided after looking at Phase 0 results.

---

## Pre-work: investigate current engine logs

Before any new code, audit what the existing `mtg-draft-runner` already records to make sure we'll be able to compute everything we need from a Phase 0 run. If anything is missing, add it before Phase 0.

### What we need to be able to extract per-draft

| Item | Why |
|---|---|
| Random seed used for pack generation | Reproducibility; needed to reuse draft pools across batches |
| Level-to-seat assignment | Required for any per-level analysis |
| Per-seat draft pool (cards opened/passed/picked) | Audit and replay |
| Per-seat final decklist (maindeck + lands + sideboard) | The "deck" output of the draft+build phase |
| Per-match result (which two seats, winner, score 2-0/2-1) | Primary data for Elo computation |
| Per-game result within a match (winner, turn count, opening hands, mulligan decisions) | Variance decomposition |
| Per-game LLM call counts and token usage by model | Cost tracking |
| Wall-clock timestamps for draft phase, build phase, each match, each game | Cost / efficiency |
| Any errors, retries, or fallback paths triggered | Data quality / exclusion criteria |

### Audit tasks

1. **Read `draft_log.rs` and `mtg-draft-runner/src/main.rs`** — list every field currently logged.
2. **Open one of the existing draft logs** (e.g. `draft-31flite-thinking.log`) — verify each item above is recoverable by parsing.
3. **Note any gaps.** Likely candidates: explicit pack-generation seed; per-game opening hand contents; mulligan decisions (these don't exist yet because mulligans don't exist yet); per-match play/draw assignment.
4. **Check that the log format is parseable** by an analysis script (regex or structured). If it's hard to parse cleanly, consider adding a structured-output mode (JSON Lines next to the human-readable log).
5. **Verify hot-reload / resume save data** includes everything we'd need to re-run a draft from any decision point.

### Output of this step

A short audit doc (one page) that says: here's what's logged, here's what's missing, here's what we need to add. This is the input to the engineering work in the next section.

---

## Engineering prerequisites

Phase 0 does not start until both of these land.

### 1. London mulligans (capped at mull-to-4)

**Engine:**
- Deal 7, prompt `keep | mull`. On mull, reshuffle and deal 7 again.
- After final keep, prompt for which N cards to bottom (where N = number of mulligans taken).
- **Hard cap at mull-to-4**: a player who has already mulled 3 times must keep their next 7 (no further mulligan offered). Avoids LLM pathological behavior at very low hand sizes.

**LLM piloter:**
- Two new decision types:
  - `mull?` — boolean (return JSON `{"thoughts": "...", "mull": true|false}`).
  - `bottom which N cards?` — list of N card indices (return JSON `{"thoughts": "...", "bottom_indices": [...]}`).
- New prompt formats for each.
- System prompt updated to describe mulligan rules and the bottoming step.

**Random / CLI piloters:**
- Random: simple keep policy (e.g. always keep, or keep unless 0 or 7 lands — pick one and document).
- CLI: prompts the human.

**Tests:**
- Unit tests for the mulligan flow including the hard cap, bottoming, and shuffle correctness.
- Integration test: a full game starts cleanly with mulligans.

### 2. Experiment runner script

A new wrapper around `mtg-draft-runner` (or extending it) that supports:

- **Mixed-level drafts** with one level per seat (already supported via `--model-N` flags — verify).
- **Latin-square seat rotation** across drafts. Generates a deterministic schedule mapping draft index → level-to-seat assignment.
- **Bo3 round robin** within each draft. Already the default for the existing runner, but verify the per-draft output is structured per match.
- **Tournament-only mode**: load pre-existing decklists and play a tournament without redoing draft+build. Critical for Batch 0b, where we want to reuse Batch 0a's decks with a different piloter.
- **Resume from checkpoint**: if a draft or match fails partway (API error, crash), the runner can resume from the last completed unit. Don't re-run completed drafts/matches when restarted.
- **Per-draft structured output**: writes a JSON or CSV file with all the fields listed in the audit section above.

**Estimated work**: ~half day after the audit and mulligan work.

### 3. Analysis script

A separate script that reads the structured per-draft outputs and computes:

- Per-level mean win rate with paired-bootstrap CI over drafts.
- Per-level observed variance `σ²_obs(L)`.
- Pairwise level differences with paired CI (within-draft `(L_i − L_j)` averaged across drafts).
- Seat-effect regression check.
- Within-pair game-noise estimate (`σ_g`).
- Plain-text summary suitable for review.

**Estimated work**: ~half day.

---

## Experimental setup (shared by both Phase 0 batches)

- **4-seat draft**, one thinking level per seat. Each seat uses its assigned level for both drafting and deck building.
- **Latin square seat rotation**: across each block of 4 drafts, every level sits in every seat exactly once. 12 drafts = 3 stacked Latin squares = each level sits in each seat 3 times.
- **Best-of-3 round robin** within each draft: 6 matches, ~14 games per draft on average.
- **Mulligans enabled** (London, capped at mull-to-4).
- **Fixed piloter** for all sides in all games — varies between batches.
- **Per-draft outputs**: structured record containing the level→seat mapping, the 4 decklists, all match and game results, and per-game logs.

### Why this setup

- **Mixed-level paired drafts** rather than mirror drafts because pairing all 4 levels in the same pack environment removes pool-quality variance from the comparison via within-draft differencing.
- **Combined drafting + deck building** because they're sequential and the natural "draft Elo" is the end-to-end output.
- **Fixed piloter** so piloting skill is held constant and the thing being measured is purely the deck the level produced.
- **Bo3** because the math (under reasonable variance assumptions) says bo3 with more drafts dominates bo5 with fewer drafts. Phase 0 will let us verify this empirically.
- **Latin square rotation** so seat-position effects average out by construction in small samples.

---

## Phase 0 — Pilot with built-in variance estimation

**Purpose**: Estimate the per-draft variance of the level estimator, sanity-check the engine and LLM piloter, get a preliminary read on level effects, and detect any failure modes. The data also tells us whether and how to expand the experiment.

### Two batches

#### Batch 0a — `medium` piloter
- 12 mixed-level drafts using 3 stacked Latin squares.
- Piloter: `gemini-3.1-flash-lite-preview` at `medium` game thinking.
- Run the **full pipeline** (draft + build + tournament).
- Save per-draft decklists for reuse in Batch 0b.

#### Batch 0b — `high` piloter
- **Reuse the 12 sets of decklists from Batch 0a** — do not run the draft phase again.
- Run **tournament only** on those decklists.
- Piloter: `gemini-3.1-flash-lite-preview` at `high` game thinking.

This is a clean **paired piloter comparison**: identical decks, only the piloter changes. It also saves the entire draft+build cost for Batch 0b, since drafting is the more expensive phase per unit of useful output.

**Total Phase 0**: 12 unique drafts, ~340 games (12 × 14 average for Batch 0a + 12 × 14 average for Batch 0b).

### Side test: LLM determinism replay

After both batches, pick **one specific matchup** (one deck pair from one Phase 0 draft) and replay it 20-30 times with new RNG seeds. Run this with both `medium` and `high` piloters.

Measures: how reproducible are game outcomes when everything else is held fixed? Isolates pure LLM stochasticity from other game RNG. Tells us how much of `σ_g` is intrinsic LLM noise vs. genuine game RNG.

Cost: ~50 games.

### Wall clock and cost

- 12 drafts × ~4 min draft+build phase ≈ 50 min (Batch 0a only)
- ~170 Batch 0a games × ~2 min ÷ 4-way parallelism ≈ ~85 min
- ~170 Batch 0b games × ~2 min ÷ 4-way parallelism ≈ ~85 min
- LLM-determinism side test: ~50 games ≈ ~25 min
- **Phase 0 total: ~4 hours wall clock, ~$25-40 in API**

### What Phase 0 produces

Per batch, computed by the analysis script:

1. **Per-level mean win rate** with paired-bootstrap CI over drafts.
2. **Per-level observed variance** `σ²_obs(L)` — the variance of L's win rate across drafts.
3. **Pairwise level differences** with paired CI.
4. **Seat-effect regression** — should be ~0.
5. **Direction sanity check**: is `high ≥ medium ≥ low ≥ minimal`?
6. **Engine sanity**: any decks at 0% or 100% win rate (suggests a bug rather than skill).
7. **`σ_g` estimate** as a free byproduct.
8. **LLM determinism**: variance across replays of the same fixed matchup.

Cross-batch:

- Does `σ_obs` differ between `medium` and `high` piloters?
- Does the level ordering replicate?
- Are pairwise differences similar in magnitude?
- Does the high piloter tighten or compress the level signal?

### Statistical analysis

**Primary estimator**: per-level mean per-draft win rate. Win rate is computed at the **match level** (a match is bo3, win = won 2-1 or 2-0).

**Confidence intervals**: paired bootstrap over drafts. 1000 resamples → percentile CIs. For pairwise comparisons, compute `(L_i − L_j)` within each draft and bootstrap the per-draft differences.

**Elo conversion** (for the final summary):
`ΔElo(L_i, L_j) ≈ 400 · log₁₀(WR_ij / (1 − WR_ij))`
with `medium = 1500` as an arbitrary anchor.

**Variance components reported**: `σ_obs` per level, within-pair game-noise component (`σ_g`), LLM-replay variance.

### After Phase 0

We review the data together and decide what to do next. Options include:
- Stop — Phase 0 is informative enough on its own.
- Expand — design and run additional drafts based on what Phase 0 reveals about variance and effect sizes.
- Iterate on the design — if something looks broken or surprising.

We are **not committing in advance** to any of these. The decision happens after looking at the numbers.

---

## Order of operations

1. **Audit existing engine logs** — produce the one-page audit doc.
2. **Add any missing logging** identified in the audit.
3. **Implement London mulligans** with mull-to-4 cap. Tests pass.
4. **Build experiment runner** — Latin square rotation, tournament-only mode, resume-from-checkpoint, structured per-draft output.
5. **Build analysis script.**
6. **Run Phase 0 Batch 0a** (`medium` piloter, 12 drafts, full pipeline). Save decklists.
7. **Run Phase 0 Batch 0b** (`high` piloter, 12 drafts, tournament-only on Batch 0a's decks).
8. **Run LLM determinism replay** (~25 min).
9. **Analysis pass**: run the analysis script, generate the summary.
10. **Review together**, decide next steps.
