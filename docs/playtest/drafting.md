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
- **A `mtg-draft-runner` run is two different harnesses in sequence.** The
  draft phase uses this crate's own prompts, schemas and backends in
  `llm_client.rs`; the tournament phase that follows hands the drafted
  decks to `mtg-player`'s game harness (`mtg-player/src/llm.rs` and
  `llm/claude_code.rs`) and plays them there. A probe that stops when the
  picks stop has tested half the program — D7 passed a full all-`cc`
  draft on the same day the tournament phase could not cast a Skaab
  Goliath at all (#398).
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
- D5 [Operator] log completeness: the log used to be the only record a
  draft left; #212 and #218 gave the runner `--seed`, `--save` and
  `--resume`, so it now has three. The reconstruction question stands and
  has gained a second half. Verify a reader with the log alone can rebuild
  the whole run — pack contents, every pick in order with the pack it came
  from, pass direction, final pools, decks, pairings and results — and
  then verify the other two agree with it: re-running the seed the log
  header records reproduces that run exactly, and a `--resume` from a
  mid-draft snapshot lands on the same position rather than a plausible
  one
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
- D9 [proposed 2026-09-04, from #202] pod-size fairness: #202 found that
  `is_c1` and `use_rare_sheet1` both key off `pack_index % 2` while
  `generate_draft_packs` deals packs with `i % pod_size`, so in an even pod
  each seat is locked to one parity and half of them can never open a mythic
  (observed `[6, 0, 3, 0, 5, 0, 3, 0]`). Generalize it: for every `--players`
  from 2 to 8, tally per seat the rare-sheet mix, the C1/C2 type and the
  variant mix over a few hundred drafts, and verify no structural property of
  the collation is constant per seat. Any per-seat constant is the same bug
- D10 [proposed 2026-09-04, from #218] operator failure handling mid-draft:
  point `CLAUDE_CODE_BIN` at a script that fails only after N successful
  calls — non-zero exit, a "usage limit reached" message, a hang — and find
  out what survives. #218 saw three failed calls in six seconds destroy a
  whole draft with a Rust backtrace and `Any { .. }` as the message, with
  nothing on disk to resume from. The fatal may well be right; the question
  is whether an hour of real drafting can be lost to a transient, whether
  the error says what happened, and whether anything is checkpointed
- D11 [proposed 2026-09-09, from #398, #399] the tournament phase on the
  subscription seat: D7 verified the draft's `claude -p` contract and
  stopped at the last pick. Take a completed draft through to standings
  with every seat `cc` (`--players 2 --best-of 1 --model cc --seed <n>` is
  about ten minutes) and audit the *games* the same way — one `claude -p`
  per decision, session continuity across the match, no metered call, and
  every prompt kind the drafted decks can reach actually answerable. The
  known break is that a schema keyed by card name is rejected outright, so
  the seat is mute at that prompt and the engine cancels the cast (#398)
  while nothing in the standings says a seat was mute (#399). Generalize
  it: enumerate the prompt kinds a limited deck reaches, reach each one on
  a `cc` seat, and treat any decision the seat cannot answer as the bug —
  a drafted deck the seat cannot pilot makes the whole tournament result
  meaningless

- D12 [proposed 2026-09-09, from #402] seeded-run identity beyond the packs:
  the draft half of a `--seed` run reproduces exactly and the tournament half
  does not, because `to_decklist` returns a `HashMap`'s iteration order and
  `DraftDeck.lands` is a `HashMap` too, so the seeded shuffle shuffles a
  differently-ordered list each process. Point `CLAUDE_CODE_BIN` at a stub
  that answers every pick, deck build and game decision from the prompt text
  alone, run the same `--seed` twice, and diff the two logs line for line
  after masking timestamps and thread ids. Audit the rest of the path the
  same way — `deck_schema_for`'s property order, `parse_deck_response`'s
  expansion of `{name: count}`, and the order the tournament spawns its
  matches in. Verify every log line and every prompt a seat is handed is
  identical between the two runs, and that anything which legitimately varies
  is recorded somewhere a reader can replay from

- D13 [proposed 2026-09-09, from #401] what else a resume launders: #401
  found the pick log, the substituted-pick counter and `--save` all skipped
  by the replay branch, so a resumed draft cannot be told from one nobody
  interrupted. Take the rest of the run's bookkeeping the same way. Set up a
  draft that fails after the deck-build phase and one that fails
  mid-tournament, snapshot each, and resume. Verify that the end-of-run token
  usage totals of a draft split across three resumes can be added back up to
  the real cost (today each run reports only its own calls), that the
  substituted-*deck* report survives a resume, that the log header records
  `--resume` and the snapshot it came from, and that a resume with a
  different `--guide-N` than the save was drafted under is refused or noted
  rather than silently producing a hybrid draft

- D14 [proposed 2026-09-09, from #404, #399] subprocess-lifecycle parity
  between the two `claude -p` backends: `mtg-draft-runner/src/llm_client.rs`
  and `mtg-player/src/llm/claude_code.rs` are two copies of one protocol and
  have now diverged in both directions — #218's wall-clock retry budget
  exists only in the draft copy (#399), and #203/#206's working timeout,
  process group, signal handlers and workdir sweep only in the game copy
  (#404). Set up one `CLAUDE_CODE_BIN` stub with a mode per failure (exit,
  `is_error`, unparsable, hang-with-descendant, hang-with-exec, SIGINT,
  SIGTERM, SIGHUP) and drive BOTH backends through every mode. Verify the
  observable behaviour is identical: the same retry policy, a timeout that
  actually returns, the same fatal wording, and after every mode no orphan
  process and no leaked scratch directory. Any difference is either a fix
  that reached one copy only, or an argument for deleting one copy

- D15 [proposed 2026-09-09, from reading `deckbuilding.rs` during D4] the deck
  answer the schema cannot describe: `parse_deck_response` pushes `count`
  copies of a name into a `Vec` before `validate_deck` ever sees it, taking
  `u32::MAX` on overflow, and `validate_deck`'s 200-card hallucination guard
  runs only over `lands`, never over the maindeck. The enum-constrained schema
  keeps a well-behaved model away from this today, which is exactly why a
  stub should go there instead. Point `CLAUDE_CODE_BIN` at maindeck counts of
  1e6 and 4e9 and at land counts of 0, -1 and 1e12, and verify each is
  refused as an invalid response — with a retry message a model could act on
  — rather than allocated first. Check the same for a deck response whose
  JSON is valid and whose keys are not the schema's

- D16 [proposed 2026-09-09, from #403] one card, one name, all the way down:
  `fallback_deck` emits its maindeck as the raw pool names, so a DFC reaches
  the engine as `"Front // Back"` with no stub involved at all, and
  `mtg-engine/src/invariants/objects.rs` explicitly tolerates that name cache.
  D4 saw one seat told `1x Grizzled Outcasts // Krallenhorde Wantons` in its
  decklist and `cleanup: Opp discarded Grizzled Outcasts // Krallenhorde
  Wantons (#8)` in its log while every board and hand line said `Grizzled
  Outcasts`. Force a fallback on a seat whose pool holds a DFC and read the
  whole game as that seat sees it — decklist section, hand, board, event log,
  every prompt that names a card, and the response schemas keyed by card name
  (#398). Verify the seat is never shown, nor asked to answer with, two
  different names for the same physical card

- D17 [proposed 2026-09-09, from #404 and the join loop in `main.rs`] which
  seat the fatal blames, and whether one seat can block the report: the pick
  loop joins the seats' scoped threads in seat order, so `Error: seat N could
  not make pack P pick Q` names the lowest-numbered failed seat rather than
  the one that failed first — and a seat that hangs is joined ahead of a
  higher-numbered seat that failed, so the run blocks instead of reporting.
  Set up a `CLAUDE_CODE_BIN` stub that keys its behaviour off the seat (the
  pick prompt's pool listing identifies it) and make seat 3 fail while seat 0
  hangs, then the reverse. Verify the message names the seat that actually
  failed, that a hang anywhere still reaches a fatal, and that the in-flight
  calls of the seats which did not fail are killed rather than orphaned when
  the run exits
