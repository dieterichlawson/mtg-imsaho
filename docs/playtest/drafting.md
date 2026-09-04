# Playtesting the draft

Subject: `mtg-draft-runner` — booster generation, the pick loop, deck
building, and the Swiss tournament that plays the drafted decks. It has
its own LLM prompts, entirely separate from the game's, and its own
notion of a correct result.

## Before you start

Two setup rules, both about money and time:

- **`--model` defaults to `claude`, a metered API seat.** Every draft
  run must pass `--model cc` explicitly. A mission that forgets is a
  mission that spent money.
- **Draft runs are long.** Eight seats is 360 picks, eight deck builds
  and a full tournament. Use the smallest `--players` the question
  tolerates and `--best-of 1`, and prefer inspecting `--log` from a small
  run over playing a big one.

The ideas below are a starting point, not a syllabus. They are what
previous nights happened to think of, and the bugs that mattered most
were usually not on the list when the night began. The real method is the
one underneath them: read the code that implements this, read the rule or
the contract it is supposed to satisfy, and find where the two disagree.
When you find a way to look that the list doesn't have, take it — and
then add it, per "Adding an idea" in `docs/playtest/README.md`.

## Where to look

- `mtg-draft-runner/src/main.rs`: pack generation, the pick loop,
  `parse_pick_response`, `build_deck_with_llm` and its fallback,
  `build_deck_prompt`, and the tournament.
- `mtg-draft-runner/src/llm_client.rs`: the draft's own system prompt,
  the pick and deck JSON schemas, the three backends, usage and cost
  accounting. `draft_log.rs` is the log format.
- `docs/isd-booster-collation.md` describes what a real Innistrad pack
  is, in enough detail to check a generated one against it — rarity
  slots, foil rate, and which cards can and cannot share a pack.
- The correctness questions here are mostly about *silence*: this
  program has fallbacks that substitute a decision when a seat's answer
  doesn't parse, and a run that quietly fell back looks a lot like a run
  that worked.

## Ideas

- D1 [Operator] pack collation and conservation: check generated packs
  against `docs/isd-booster-collation.md` — rarity slots per pack, the
  foil slot's rate, no card twice in one pack, and the conditional
  structure the C1/C2 sheets imply over a few hundred packs. Then check
  conservation across a draft: packs shrink by exactly one per pick,
  passing alternates left/right/left by pack round, each seat sees each
  pack exactly once, and every card printed into a pack ends in exactly
  one seat's pool. `filter_implemented` drops unimplemented cards before
  packs are built — quantify what that removes and whether it skews the
  rarity or colour balance the collation intends
- D2 [Vandal] the silent first-card pick: `parse_pick_response` falls back
  to `available[0]` when a seat's response doesn't parse. Point
  `CLAUDE_CODE_BIN` at a wrapper returning junk, an out-of-range index,
  valid JSON under the wrong key, and an empty string, and find out
  whether that fallback is visible anywhere — a log line, a warning, a
  counter in the usage summary. A seat that quietly took card 0 forty-five
  times must not be indistinguishable from a seat that drafted
- D3 [Vandal] the deck-build fallback: force ten consecutive invalid deck
  responses and inspect what the fallback builds — the entire pool as
  maindeck plus 9 Island and 8 Swamp regardless of what colours the pool
  is. Then follow it downstream: does the log, the tournament, and the
  standings present that deck as a legitimately built one, and does a
  62-card off-colour deck even play?
- D4 [Rules Lawyer] deck legality and identity: every built deck legal for
  limited — size, only cards from that seat's own pool, basics unlimited,
  DFC and split names counted once — and the decklist that gets played in
  the tournament identical to the one the log says was built
- D5 [Operator] log completeness: `main` seeds from `rand::thread_rng()`
  and there is no `--seed`, so the log is the only record a draft leaves.
  Verify a reader with the log alone can reconstruct the whole run — pack
  contents, every pick in order with the pack it came from, pass
  direction, final pools, decks, pairings and results — and file what is
  missing
- D6 [Handler] draft prompt sufficiency: what a drafting seat is told
  against what it needs to pick well. Is its pool so far shown at every
  pick, with colours and curve, or only the pack? Does it know the pack
  round and pick number and which way packs are passing? Does the
  deck-building prompt carry anything but names and counts
  (`build_deck_prompt` sends a name/count list; oracle text is in the
  system prompt's card reference)? This is H5's question asked of the
  draft harness
- D7 [Handler] draft subprocess contract: with `CLAUDE_CODE_BIN` pointed
  at a logging wrapper, one `claude -p` per pick, session continuity
  across a seat's 45 picks, `--model-N` actually routing to seat N, the
  end-of-run usage totals and cost summary consistent with the number of
  calls, and — the one that matters — no metered API call made at all
  when every seat is `cc`
- D8 [Competitor] tournament integrity: run a small Swiss tournament and
  check its bookkeeping — no pairing repeated, an odd player count handled
  honestly, match results and standings arithmetic correct, play/draw
  alternating between games of a match, and each seat playing its own
  drafted deck
