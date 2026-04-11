# Task: Engine and draft-log fixes for the Draft Elo experiment

You're working in `/Users/dlaw/mtg`, a Rust workspace implementing a Magic: The Gathering engine plus AI players. We're preparing to run an experiment that estimates Draft Elo for different LLM thinking levels by running many drafts and tournaments and analyzing the resulting logs. An audit of the current logging surfaced several issues — both real correctness bugs and missing structured information — that need to be fixed before the experiment can run.

The full experiment plan is in `DRAFT_ELO_EXPERIMENT_PLAN.md`. Read its "Pre-work: investigate current engine logs" section for the audit checklist context, but everything you need to actually implement is in this prompt.

**Out of scope:** London mulligans are being implemented in a separate task. Don't add or block on mulligan support here. However, your structured `GAME_RESULT` log entry should leave room for a `mulligans_taken` field per player so that the mulligan work can populate it without changing the format again.

---

## What needs to change, in priority order

### 1. Fix the play/draw bug (highest priority — this is a real correctness issue)

**Current behavior:** In `mtg-engine/src/engine.rs`, `setup_game` (around line 3121) hardcodes the active player to `p0` for turn 1. There is no coin flip, no first-player choice, no alternation across games in a match. Confirm this by reading the function and grepping for `active_player` in `engine.rs`.

This means in any best-of-3 match, the deck assigned to `seat_a` always has the play advantage in every game. Going first is worth ~5-7 percentage points in Magic, so this is a systematic ~5-7 pp bias on every match outcome. It must be fixed before any data is collected.

**Required fix:**

- `engine::setup_game` should accept an explicit `first_player: PlayerId` parameter (or the equivalent on `GameConfig`). Pick whichever is cleaner; document the choice.
- The first turn marker (`── Turn 1 (pN) ──`) and any internal `active_player` initialization should respect this parameter, not hardcode `p0`.
- The `mtg-draft-runner` (`mtg-draft-runner/src/main.rs`, look at `play_match` and `play_game`, around lines 589-718) needs to:
  - **Game 1 of each match:** randomly choose the first player, using a per-match RNG that itself derives from the top-level seed (see fix #3). Log the choice.
  - **Games 2 and 3:** the loser of the previous game in the same match is on the play (standard MTG loser-on-the-play rule).
- The `mtg-runner` (`mtg-runner/src/main.rs`) — the 1v1 runner used for ad-hoc games — should also be updated. For that runner, the simplest correct behavior is a random coin flip per game. Don't try to share match-state with it.
- All existing tests that assume `p0` always starts must be updated. Search for tests that depend on this and either parameterize them or pin them to `first_player = p0` explicitly.

**Validation:** add a unit test that runs `setup_game` twice with different `first_player` values and confirms the resulting `active_player` and the first-turn log line differ. Also add a test in the draft runner that confirms the loser-on-the-play rule across games of a match (you can use scripted/random players for this).

### 2. Add `GAME_START` markers

**Problem:** The current `MATCH` and `GAME N (Seat A vs Seat B)` log entries are written **after the round finishes** in a tight burst on `ThreadId(1)`. The actual `PROMPT` / `RESPONSE` / `THOUGHT` / `CHOSE` lines from inside live games are streamed across parallel worker threads with no round/match/game annotation. ThreadId is *almost* enough to disambiguate, but threads get reused across rounds, so a parser can't use ThreadId alone.

**Required fix:** Emit a structured `GAME_START` marker at the start of every game, **from inside `play_game` in `mtg-draft-runner/src/main.rs`**, on the same thread that will run the game. The marker must include enough fields to attribute every subsequent log line on that ThreadId (until the next `GAME_START` or `GAME_RESULT` on that thread) to this specific game.

**Required fields:**
- `round` (the tournament round number)
- `match_seat_a`, `match_seat_b` (the two seats playing this match)
- `game_num` (1, 2, or 3 within the match)
- `seat_on_play` (the seat that is on the play in this game)
- `seed` (the per-game shuffle seed — see fix #3)

Use a `log_game_start!` macro added to `mtg-draft-runner/src/draft_log.rs`, matching the existing macro style there. The marker label should be a single grep-friendly token like `GAME_START`. The body should be a structured key-value block (one key per line, like the existing entries) that's both human-readable and easy to parse.

You will need to plumb `round` into `play_match` and `play_game` (currently they don't know it). Add it as a parameter.

**Do not** remove the existing post-round `MATCH` and `GAME` block writes — they're still useful as the structured "after the round" record. The new `GAME_START` is a per-thread live marker, complementary to those.

**Do not** change `mtg-player/src/llm.rs`'s `SYSTEM` logging. Leave it as it is.

### 3. Capture and log RNG seeds

**Problem:** Both `mtg_draft::pack::generate_draft_packs` and `mtg_engine::engine::setup_game` use `rand::thread_rng()` directly with no seed exposed. This means:
- Draft pools cannot be reproduced from logs.
- Game shuffles cannot be reproduced from logs.

**Required fix:**

- **Top-level seed:** Add a `--seed <u64>` CLI flag to `mtg-draft-runner`. If not provided, generate a random `u64`, log it, and use it. Either way, log the chosen seed in the `HEADER` block at the top of every run.
- **Pack-generation seed:** Plumb the top-level seed (or an RNG derived from it) into `generate_draft_packs`. The function signature already takes `&mut rng` — change the call site to pass a seeded `StdRng` (or whatever the project uses) instead of `thread_rng`. Document the derivation rule (e.g. "pack rng = StdRng::seed_from_u64(top_seed)").
- **Per-game shuffle seed:** Derive a deterministic per-game seed from the top-level seed plus identifying info (e.g. `(top_seed, round, match_idx, game_num)` hashed). Pass this seed into `setup_game` (or into a new `setup_game_with_seed` if you'd rather keep the existing API). Log the per-game seed in the `GAME_START` marker (fix #2).
- **Per-match coin flip seed:** Derive similarly so the game-1 first-player choice is reproducible.

The goal: given `--seed N`, two runs of the experiment with the same arguments should produce byte-identical packs, identical first-player choices, and identical shuffles. (LLM responses are still nondeterministic, of course — we're not asking for end-to-end determinism, just for the *random* parts to be reproducible.)

Verify this with a unit test: call `generate_draft_packs` twice with the same seed, assert byte-identical output. Same for `setup_game` library order.

### 4. Add a structured `GAME_RESULT` entry

**Problem:** The post-round `GAME N (Seat A vs Seat B)` block embeds the engine state log as a text blob. To find the winner or turn count, you have to parse the embedded text. We want a structured field next to the existing block.

**Required fix:** After each game finishes (in `play_game` in the draft runner), emit a `GAME_RESULT` log entry with structured fields:

- `round`, `match_seat_a`, `match_seat_b`, `game_num` (same identifiers as `GAME_START`)
- `winner` (`"seat_a"`, `"seat_b"`, or `"draw"`)
- `turns` (final turn number)
- `seat_on_play` (so the parser doesn't have to look back to `GAME_START`)
- `mulligans_a` and `mulligans_b` (always `0` for now — leave the field present so the mulligan task can populate it without changing the format)
- `tokens_input`, `tokens_output`, `tokens_cache_read`, `tokens_cache_create` (per-game delta — see fix #5)

Same macro style as the other log helpers. Same single-token grep-friendly label (`GAME_RESULT`).

Do **not** remove the existing `GAME N (Seat A vs Seat B)` block — it's still useful for human reading. `GAME_RESULT` is a separate structured complement.

### 5. Per-game token usage

**Problem:** Token usage is currently aggregated globally in `mtg-player/src/llm.rs` (look at the `record_*_usage` functions and the static `LLM_MODEL_USAGE` map) and printed once at the end of the run via `mtg-draft-runner`'s `print_usage_summary`. There's no per-game breakdown, so we can't see if a particular game blew up in cost.

**Required fix:**

- Add a snapshot mechanism to the global usage tracker: a function that returns a clone of the current per-model totals.
- In `play_game`, snapshot the totals before the game starts and again after it ends. The delta is the per-game cost.
- Include those deltas in the `GAME_RESULT` entry (fix #4) as the `tokens_*` fields.
- Don't change the existing global aggregation — it should still work end-to-end.

This is the cheapest fix on the list once #4 is in place.

---

## Things to be careful about

These are CLAUDE.md rules and project memory, follow them strictly:

- **Never take silent shortcuts.** If you find that one of these fixes is harder than it looks (e.g. plumbing seeds through `setup_game` requires a wider refactor), surface that explicitly rather than papering over it. Don't half-implement.
- **Engine limits are real issues.** If the current architecture makes one of these fixes structurally hard (e.g. RNG is buried somewhere that can't be parameterized cleanly), flag it as an ISSUE in your report, don't excuse it.
- **Correctness over convenience.** Code must work for the right reasons.
- **Small incremental commits.** Each numbered fix above should be its own commit (or a small series of commits). Each commit should compile cleanly with `cargo check` and have **zero warnings**.
- **Don't add unrelated changes.** Don't refactor or "improve" code outside the scope of these fixes.
- **Don't touch `llm.rs`'s `SYSTEM` logging.** Even though the per-game system prompt is bloated, we're keeping it as a redundant "new game" marker.

## Definition of done

- `cargo check` passes with zero warnings on all crates.
- All existing tests still pass.
- New unit tests for: deterministic pack generation given a seed, deterministic library shuffle given a seed, `setup_game` respecting `first_player`, loser-on-the-play within a match.
- A test run of `mtg-draft-runner --seed 42 --players 4 ...` produces a log file containing: `seed=42` in the `HEADER`, a `GAME_START` block per game with all required fields, a `GAME_RESULT` block per game with all required fields, and reproducible pack contents across two runs with the same seed.
- Short summary of what changed, what files were touched, any issues you flagged but did not fix, and how to verify the play/draw fix on a fresh log.

## Where to start

1. Read `CLAUDE.md` and `DRAFT_ELO_EXPERIMENT_PLAN.md` for context.
2. Read `mtg-engine/src/engine.rs` around `setup_game` (~line 3121) to understand the current setup flow and what needs to change for first_player.
3. Read `mtg-draft-runner/src/main.rs` `play_match` and `play_game` (~lines 589-718) to understand how matches and games are currently sequenced.
4. Read `mtg-draft-runner/src/draft_log.rs` to understand the macro pattern for new log entries.
5. Read `mtg-player/src/llm.rs` `record_anthropic_usage` / `record_gemini_llm_usage` and the global usage map to understand where to snapshot tokens.
6. Look at recent commits (`git log --oneline -20`) for examples of how similar features were structured.
7. Sketch the design (function signatures, struct changes, log format) before writing code. Confirm the design with a short note in your final report so the format is documented.

Ask if anything in the spec is ambiguous before committing to a design.
