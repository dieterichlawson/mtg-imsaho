# Playtesting the draft

Subject: `mtg-draft-runner` — booster generation, the pick loop, deck
building, and the Swiss tournament that plays the drafted decks. It has
its own LLM prompts, entirely separate from the game's, and its own
notion of a correct result.

## Before you start

Two setup rules, both about money and time:

- **`--model` defaults to `claude`, a metered API seat.** Every draft
  run must pass `--model cc` explicitly. A mission that forgets is a
  mission that spent money. `cc:claude-haiku-5-5` spends the least of the
  quota and picked closest to a stronger model's picks on real packs
  (`reports/llm-cost.md`, phase 4).
- **Point `CLAUDE_CODE_BIN` at an ABSOLUTE path.** The backend runs each call
  in the seat's scratch `current_dir`, while the up-front "is the CLI runnable"
  check runs in the invocation cwd — so a relative path passes the guard and then
  fails every single call with `os error 2`, burning the whole wall-clock retry
  budget before the run dies (#540). Both 2026-09-18 probes lost time to this.
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
- **Multi-human table.** `mtg-draft-server` (`mtg-draft-runner/src/bin/`)
  hosts the same draft for people and AI seats together; `docs/draft-with-friends.md`
  is the manual. The asynchronous table is `mtg-draft/src/table.rs`, the
  phases and what each seat is shown are `mtg-draft-runner/src/lobby.rs`,
  the threads and the sockets are `src/server.rs`, and the match loop both
  runners share is `src/game.rs`. Its `--log` is this same format, so the
  readers above apply: a person's pick logs `human` where the prompt and
  response go, the table's own picks log `auto-pick: <why>`, and a
  forfeited match (a kicked seat) is marked as the runner marks a stalled
  one. A test drives the whole thing under a stub `claude`
  (`mtg-draft-runner/tests/lobby_*.rs`), which is the cheap way to probe it.

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
  meaningless.
  **Run 2026-09-13, and three of the nine shapes are still unreached on a real
  seat.** A 2-seat ISD pod reaches nine request shapes (`pick`,
  `maindeck`+`lands`, `action`, `confirm`, `mull`, `indices`, `card_indices`,
  `order`, and the per-blocker assignment object); one real 14-turn game reached
  six of them and answered all six. The three left over are the board-dependent
  ones — `confirm` (the concede check), `card_indices` (the cleanup discard,
  which is also the smallest prompt in the game, #463) and `order` (trigger
  ordering, CR 603.3b). Reaching them on a real seat is not a matter of playing
  longer: it wants a pod whose *pools* force them, so pick the seeds by drafting
  under a `CLAUDE_CODE_BIN` stub first and grepping the captures for the shape,
  then re-run those seeds with `--model cc`. The two watchdog defects the night
  did find (#488, #489) mean a seat that cannot answer one of these three spins
  silently rather than reporting, so an unreached shape is exactly where the
  cost lands

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
  **Run 2026-09-18, and the `to_decklist` half is confirmed fixed.** Five seeded
  pairs under a stub that is a pure function of (`--json-schema` argv, prompt):
  the draft half, the decks, the games, the standings and every prompt in every
  `claude -p` session are byte-identical (440 and 818 stub invocations per run,
  empty multiset diff, every per-session stream matching), and all four named
  ordering surfaces are deterministic — `serde_json` is built *without*
  `preserve_order` (`Cargo.lock` lists no `indexmap` under it), so `Map` is a
  `BTreeMap` and every request map is lexical for free, which also makes
  `deck_schema_for`'s explicit sort redundant and `parse_deck_response`'s
  expansion order lexical. What is *not* reproducible is the log, from
  `DECK BUILDING` onward: `log_deck_building!` is called inside `s.spawn` and the
  tournament's `PROMPT`/`RESPONSE` records are written inline from each match
  thread, so the same seed diffs to 1,047 hunks at `--players 4` with an
  identical sorted line multiset (#541); the session ids are the one genuinely
  varying value and nothing records them (#542). Two methodological notes for
  anyone re-probing determinism here. First, **`--players 2 --seed 41` passes by
  luck** — it is a two-way race that fell the same way twice, and three of the
  other four pairs failed, so a determinism claim needs at least three seeds and
  one `--players 4`. Second, **`diff` alone cannot tell "different run" from
  "same run, different line order"**: pair it with `diff <(sort a) <(sort b)`,
  which is empty for an ordering defect and non-empty for a real one

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

  **Run 2026-09-23, and #401's fix holds — the laundering moved to the money and
  the flags.** Everything #401 named is now correct and was verified end to end:
  the header carries `resumed from: <path> (N picks replayed)`, all 84 picks
  appear as `PICK … — replayed from the snapshot, no prompt sent`, a snapshot
  holding 20 substituted picks resumes into `=== Substituted Picks === 20
  pick(s)` plus 20 `WARN Pack` log lines, `--save` keeps writing after a resume,
  and a resumed run's snapshot resumes again (a three-process chain works). The
  substituted-*deck* report also survives, but only because deck building is
  re-run from scratch every process — it is re-derived, not carried. What does
  not survive is the cost. `print_usage_summary` is reached only on the happy
  exit, so `die` and a SIGKILL publish nothing at all, and `MODEL_USAGE` is
  process-local: one `--seed 7` draft split across three runs made 288 successful
  `claude -p` calls and reported 238, and the surviving run prints `Draft: 36
  calls` in the same file whose log holds 84 `PICK` records (#578; seed 23 in two
  runs: 288 real, 248 reported). `DraftSave` records `{seed, set, players, picks}`
  and the reconciliation loop checks exactly those three, so `--guide`/`--guide-N`
  and `--model`/`--model-N` are taken from whatever was typed: an alpha-guide
  snapshot resumed under beta exits 0, silently, with the header claiming
  `guide: beta.txt` for a draft 20 of whose picks were alpha's (#579). Two
  adjacent ones: a resume that grows the pod pads `guides` with `None` while
  `models` is padded with `models[0]`, so a global `--guide` reached seats 0-1 of
  a 4-seat resumed draft and not seats 2-3 (#580), and the reconciliation note
  fires against the *generated* default seed, announcing a 19-digit number nobody
  passed (#582). The idea's framing also has a gap worth carrying forward:
  `--save` checkpoints the pick loop only, so a kill just inside the tournament
  re-bills the deck build and every game — 49 of 288 calls bought twice at
  `--players 2 --best-of 1`, and the tournament is ~95% of a default run's calls
  (#581). Method notes for a re-probe: a stub whose successful calls draw from a
  *shared* counter file makes the addition exact, and the fail counter must be
  per-run while the token counter is global; and a tournament-phase failure
  cannot be forced by stub exit codes, because the game harness rejects the
  answer and plays on (#488/#489), so kill the pid you spawned once `Starting
  Swiss tournament...` appears.

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

  **Run 2026-09-23, and the process half is in parity; the reporting half is
  not.** One stub with fourteen modes through both backends: exit 7, `is_error`,
  junk, empty, hang-with-descendant, hang-with-`exec`, answer-then-linger on
  stdout and on stderr, a result with no `structured_output`, a 20-second
  transient outage, and SIGINT/SIGTERM/SIGHUP — including one delivered during a
  draft's *tournament* phase. Every process-level property matched: the per-call
  timeout fires at the configured value on both (`5001ms` at
  `MTG_CLAUDE_CODE_TIMEOUT_SECS=5`), the wording of every non-fatal line is
  identical apart from the deliberate `attempt 1` vs `attempt 1/3`, all three
  signals exit 130/143/129, and there were **zero orphaned stub processes or
  descendants on either side in any mode**. The scratch sweep does cover both
  prefixes now: a fatalled draft leaks its two `mtg-draft-claude-code-<pid>-*`
  directories, because `die` → `process::exit` runs no destructor, and the next
  run's `prepare_seat` removes them. `claude_code_run` being one function did
  its job, and #404 is genuinely closed. What still differs is the reporting,
  three new ways: `API_RETRY` is written at `INFO` by `claude_code.rs:447` and at
  `ERROR` by `llm_client.rs:750`, so `grep ERROR` over a game log finds no
  usage-limit retry (#583); the draft fatal prints the configured budget where
  the elapsed time belongs — "gave up after 1 attempts over 6s" after 60 seconds
  (#585); and the retry policy itself is still #399's deferred judgment call,
  now with a number — a 20-second outage costs the game seat 2 of 74 decisions
  to the fallback and the draft seat none, and a call that *succeeds* with prose
  and no `structured_output` gets zero retries from the game copy against
  retry-then-fatal from the draft copy (74/74 decisions substituted, no stderr,
  exit 0). The one thing the mode list above does not name is where the night's
  biggest find was: `available()`'s `--version` probe is the only `claude -p`
  subprocess outside `run_print_mode`, with no timeout, no `setpgid` and no
  `LIVE_GROUPS` slot, so a CLI that does not answer `--version` hangs **both**
  runners forever before the first prompt with no message, and `kill -INT` or
  `-TERM` leaves the probe and its descendant at `PPID=1` (#584). Two setup
  notes for a re-probe: send signals with `kill -INT <pid>` and never Ctrl-C at
  a terminal, because an interactive Ctrl-C goes to the whole foreground process
  group and hides exactly this class of defect; and take the runner's real pid
  rather than `$!` when the command is wrapped in `timeout`, which forks — the
  workdirs are named with the *binary's* pid, so an orphan hunt keyed on the
  wrapper's finds other testers' directories instead of your own.

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

  **Run 2026-09-18, and the maindeck is the one number with no bound on it.**
  A count of 4e9 takes the process down with an allocator abort (exit 134) after
  every pick has been paid for, because the `{name: count}` expansion allocates
  before `validate_deck` runs and the 200-card guard covers `lands` only (#535);
  a count `as_u64()` cannot read — `1.0`, `"1"`, `{count: 1}`, `-5` — is
  rewritten to zero and the card dropped under `accepted` / `DECK (40 cards, 0
  retries)`, with the maindecked card sitting in the sideboard (#536). Non-schema
  keys and a 1e12 *land* count are both refused correctly, so the retry loop
  works where it is reached. The comparison that made the case is the one the
  README's four-surfaces rule asks for: `parse_pick_response`, in the same crate,
  takes identical malformed input and is safe and loud about it — `?` out rather
  than `unwrap_or(0)`, bounds-checked, `Pick::Substituted` plus a WARN and a
  counter (#195). What is left here is the other direction: sweep the rest of the
  program for a number a seat supplies that is used before it is checked.
  `deckbuilding.rs:236` was the only model-number-driven allocation in the three
  crates tonight (`grep 'for _ in 0\.\.' mtg-player/src/llm.rs mtg-draft-runner/src
  mtg-draft/src`), but the game harness takes only indices today and would not
  stay safe if a prompt ever asked for a count
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

  **Run 2026-09-23, and it comes back NEGATIVE — the name chain holds on every
  surface.** Two `--players 2 --best-of 1 --seed 7` runs under a capturing stub,
  one with valid decks and one answering the deck prompt with no `maindeck` key
  so `parse_deck_response` refuses all ten attempts and both seats take the
  fallback; both 42-card pools held three DFCs and Seat 0's fallback deck
  maindecked all three. Extracting all 20 full DFC names from
  `data/sets/isd.json` and searching them as exact strings found **0** in 1,886
  captured prompts, 1,886 schemas, 1,886 system prompts and both run logs, with
  0 illegal top-level schema keys alongside. The only ` // ` a seat meets is the
  deliberate two-face headline, byte-identical in the pack line, the pool
  listing and the deck-build prompt because all three go through
  `card_lines.rs`. Both producers of a `DraftDeck` front-face the maindeck now
  (`deckbuilding.rs:193` and `:440`); the sideboard does keep raw pool names in
  the struct, but `draft_log.rs:282` front-faces it on the way out, so the raw
  string reaches no reader. `deck_schema_for` keys `maindeck` by front face
  only, and a back-face answer is refused correctly (`'Howlpack Alpha' is not in
  your drafted pool.`). Two things worth recording as **checked, correct** so
  nobody re-files them: the `--save` snapshot holds raw set-data names, which is
  the pool's native form and normalises through `front_face` on read; and a seat
  *can* connect the two names when a card transforms, because its decklist
  section carries both faces with both faces' rules text and the event log
  writes `Ulvenwald Mystics (#1) transforms into Ulvenwald Primordials (#1)`
  with one object id on both sides (#500), 42 times in one game. The idea's
  last clause was also answered in passing: the two distinct `## Your decklist`
  sections across 1,102 game system prompts match the two logged 40-card
  fallback decks entry for entry, so the tournament plays the deck the log says
  was built. There is nothing left here; a re-probe wants a *new* producer of
  card-name strings, not this one.

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
  **Run 2026-09-18: the blame is survivable, the cleanup is not.** Both of the
  claims above are half wrong and worth correcting in place. `enumerate()`'s
  index *does* equal the seat, because `pick_inputs` is built `(0..args.players)`
  in order, so a lone failure is named correctly — the real defect is
  first-`Err`-wins in seat order (seat 3 fails at t≈0.2s, the headline blames
  seat 0) plus a report that waits on every lower seat, dead at t≈2.4s and silent
  until t=45s, ~600s at shipped defaults, while `API_FATAL` carries a thread id
  and no seat at all (#539). And a hang does *not* block forever: the per-call
  watchdog and the retry budget bound every call. What the idea did not ask about
  is where the night went — `die` → `process::exit(1)` runs no destructors, so a
  fatal in one seat orphans every other seat's `claude -p` tree to init at
  `PPID=1` (#537), and `LIVE_GROUPS`' four slots, commented "more seats than a
  run has" because they were written for a 2-seat game, lose four of a default
  8-seat draft's calls on the signal path too (#538). Setup notes for a re-probe:
  the pick prompt states `You are seat N of M` (`llm_client.rs:1124`) and is the
  reliable seat discriminator — a pool listing is *not*, since every pool is
  empty at pack 1 pick 1 — the deck-build prompt states no seat, so memoise
  seat→`--session-id` during the picks; and the two knobs that make a failure
  arrive quickly are `MTG_DRAFT_RETRY_BUDGET_SECS` (`llm_client.rs:618`) and
  `MTG_CLAUDE_CODE_TIMEOUT_SECS` (`mtg-player/src/llm/claude_code.rs:40`, default
  300s)
- D18 [proposed 2026-09-09, from #398 and #404] the draft's own copy of the
  harness contract. `mtg-draft-runner/src/llm_client.rs` builds its own
  prompts, its own schemas and its own `claude -p` invocation, and #404 is
  what that costs: the game backend's hang fix never reached this copy.
  #398 is the same shape on the schema side — a top-level property key the
  API refuses is a 400 before the model reads anything, and the seat cannot
  tell that apart from an answer it did not like. The deck-build schema keyed
  by CARD NAME, legal only because it nested them under `maindeck` (it
  answers one count per numbered pool row since #867); the pick schema uses
  an index enum. Neither is guaranteed by anything but a
  `debug_assert`. Walk the two schema builders and the prompt text against
  the game side's (`mtg-player/src/llm.rs`) and list every place they have
  drifted — retry counts, timeouts, session handling, how a refusal is told
  from a bad answer, what happens to a schema the API rejects. Each
  difference is either a fix that did not travel or a decision nobody wrote
  down. Then ask D6's question of the deck-build prompt specifically: 45
  cards keyed as 45 schema properties is a large object for a model to fill
  in correctly, and a wrong count there is a deck that fails legality rather
  than a deck that plays badly

  **Run 2026-09-28, and the drift that costs something is #213: it never travelled.**
  `ClaudeCodeDraftBackend::decide` runs every pick and deck schema through
  `sanitize_schema_for_anthropic` (`llm_client.rs:399`), which has no `keep_thoughts`
  parameter and deletes the `thoughts` property and its `required` entry
  unconditionally — while the game copy calls `sanitize_schema(schema, true)` under a
  comment that names #213 and says "This is the seat the project actually runs". On
  the wire in one `--players 2 --best-of 1 --model cc` run: **86 of 86 draft calls got
  a schema with no `thoughts`; 202 of 202 game calls got one with it.** And
  `STRUCTURED_RESPONSE_FORMAT`, appended to every draft backend, tells the seat its
  reasoning "happens in the model's extended-thinking channel" and that a `thoughts`
  key will be rejected — true of the Anthropic and Gemini backends, false in both
  halves for `claude -p`, which has no thinking block the harness reads (#213's own
  finding 3). Measured with a stub that emits only the keys its schema declares, the
  way a constrained decode does: **0 reasoning records across all 86 draft calls, 202
  across the tournament's, in the same run, log, binary and seat**; a pick's whole
  durable account is `{"pick": 13}` (#607). Two things **checked, correct**: both
  copies guard #398's top-level-key rule with `debug_assert!`, which is a stated
  "loud for a developer, retry path for a live game" choice in both comments and not
  an accident of one; and the deck-build prompt is no longer #487's name/count list —
  it states the 40-card floor, the 17/23 target, the two-colour norm, that the
  leftovers are a sideboard nothing is swapped from, and where a basic land goes, with
  `max_retries = 10` and each retry prefixing the validator's own message. The
  deck-object-size half of the idea (45 keys for a model to fill in correctly) is
  **not** done: nothing here drove a real seat at a full 45-property `maindeck`

- D19 [proposed 2026-09-13, from #484 and the D8 night] the drawn game, which no
  probe has ever reached: `play_match`'s `while wins_a < wins_needed && wins_b <
  wins_needed` counts wins, never games, so a draw is a free extra game and MTR
  §6.5's three-game cap does not exist. D8 could not force one — the engine's
  only draw is `sba.rs`'s "zero players alive", a simultaneous loss. Set one up
  deliberately: find or write the pair of ISD decks that can kill both players in
  one SBA check (symmetric damage, or a mutual empty-library draw), confirm it
  draws under `mtg-runner` first, then feed the same decks to the tournament with
  a `CLAUDE_CODE_BIN` stub that steers toward it, at `--best-of 3`. Verify the
  match stops after three games played and is recorded as a draw —
  `MatchResult::winner()` returning `None`, `Standing::match_draws` incremented
  for both seats, `(Draw)` in the MATCH log line, and the draw visible in the
  standings — rather than a fourth game. Then ask the same question of
  `--best-of 2` and `--best-of 5`, where `best_of / 2 + 1` and `MatchFormat`'s
  `n.min(3)` already disagree about how long a match is

  **Run 2026-09-28: #484's fix holds, and the drawn MATCH is reachable — through an
  even `--best-of`, not through a drawn game.** The idea's premise has aged: the loop
  is `match_is_over(best_of, games.len(), wins_a, wins_b)` with MTR 6.5's
  games-played clause, and `MatchFormat`'s `n.min(3)` is gone. The drawn *game* is
  still unreached and looks unreachable from this pool, which is worth writing down so
  nobody builds the deck pair again: the engine's only draw is `sba.rs:369`'s "zero
  players alive", **no card in `data/oracle_cache.json` deals damage to each player or
  makes each player draw**, so there is no simultaneous loss to construct, and 12
  `random` vs `random` games at seeds 1-31 ended 9 on life total and 3 on an empty
  library with none simultaneous. But the match-level path needs no drawn game at all:
  `wins_needed` is `best_of / 2 + 1`, so at an even `--best-of` a level score is a
  draw. `--best-of 2` stopped every match at 2 games and `--best-of 4` at 4, and the
  whole chain the idea asks for fired — `MatchResult::winner()` `None`, `MATCH Round 1
  — Seat 0 vs Seat 1: 2-2 (Draw)`, `match_draws` incremented for both seats, `0-0-2`
  in the standings, `match_points` 2. 1/3/5 play 1/3/5 games with the right "Games 2 to
  N" prose, so #210 is closed too. What the probe found instead is #609: `play_match`
  re-inits both conversations per game and passes only the constant
  `MatchFormat::BestOf(n)`, so hashing every game-phase `--system-prompt` in a
  `--players 4 --best-of 4` run gives **3,232 calls and 4 distinct prompts, 808 each —
  one per seat** — identical between game 1 and game 4 and between a 0-0 match and a
  2-1 one. Method note: **a conceding stub is what makes this affordable** (D22's
  note), and an even `--best-of` is the cheapest drawn match anyone will find

- D20 [proposed 2026-09-13, from #485 and #481] does the prompt change when the
  draft does: the runner varies the draft by `--players` (2..8), `--set`,
  `--guide-N` and `--resume`, and #485 found the `## How drafting works` section
  byte-identical at `--players 2` and `--players 8` while describing an 8-pod,
  while #481 found a `--resume`d seat opening a fresh `claude -p` session
  mid-pack-round with no pool in the prompt and no session behind it. Set up a
  `CLAUDE_CODE_BIN` stub that writes argv + stdin + schema to one file per call,
  and run the same `--seed` at every `--players` from 2 to 8, with and without
  `--guide`, and once from a mid-pack-round snapshot (take a `--save`, truncate
  its `picks` array in place, `--resume` it). Diff the system prompt and the
  per-pick prompts across the runs. Verify that every structural difference the
  runner actually implements is visible somewhere in what the seat is told — the
  pod size and the seat's own index, how many picks until a pack wheels and how
  depleted it will be, which seat's guide is in force — and that anything the
  seat is relied on to remember rather than be told (its pool, its picks so far)
  is either restated in the prompt or provably still in that seat's session. A
  configuration the runner supports and the prompt does not mention is a seat
  reasoning about a different draft than the one it is in

  **Run 2026-09-28, and #485 and #481 both hold — the gap is the pod below the range
  #485 was written for.** Ten runs at `--seed 7` under a capturing stub, `--players`
  2-8 and 12: one distinct system prompt per seat at every pod size, each carrying its
  own index and the real pod size, and the wheel prose arithmetically right rather than
  merely parameterised (at 8 seats, "8 picks later, with the 7 cards the other seats
  took gone" against a measured 14 cards at pick 1 → 6 at pick 9). Every `after your
  pick this pack passes <DIR> to seat N` claim was checked against the pack the named
  seat was actually handed next: **1,365 handoffs across pods 2-8 and all three pack
  rounds, 0 real mismatches**, and `Pick N of 14` / `Available (15-N cards)` agreed
  1,974 of 1,974. `--guide` with `--guide-1`/`--guide-3` routed GLOBAL/ALPHA/GLOBAL/BETA
  to seats 0-3 with no bleed, and a `--save` truncated in place to 24 picks (pack 1
  pick 6, mid-pack-round) resumes into a first prompt that already restates a 6-card
  pool, tracking `(pack-1)*14 + pick-1` for all 144 remaining picks. The one
  configuration whose prompt describes a different draft is `--players 1`, which
  `count()` accepts (it refuses only 0): the seat falls into `Table::description`'s
  `else` arm and is told it is "drafting against 0 other seat(s)", to read signals off
  cards nobody upstream took, and 42 times that "this pack passes LEFT to seat 0", then
  the log prints an empty `TOURNAMENT` header into `1. Seat 0 — 0-0 (0 game wins)`
  (#608). Two method notes. **Compare pack residues as sorted multisets** — an ISD pack
  can hold two copies of a common and the runner does not remove the copy your index
  names, which is 1 of my 1,365 compares and is the checker's bug, not the runner's.
  And a `--save`'s `picks` array truncates cleanly with `json.load`/`json.dump`; `round`
  and `pick` are 1-based, so filtering on them lands you exactly mid-pack-round

- D21 [proposed 2026-09-18, from #537 and #538] every exit path, not just the
  signal one: #206 killed in-flight `claude -p` trees on SIGINT/SIGTERM/SIGHUP,
  and D17 found that the *fatal* exit (`die` → `process::exit`) sweeps nothing at
  all, and that `LIVE_GROUPS`' four slots lose four of a default eight-seat
  draft's calls even on the signal path that was fixed. Walk the remaining exits
  the same way, with a `CLAUDE_CODE_BIN` stub that sleeps 600s in some seats and
  an **absolute** path (a relative one silently fails every call after the first,
  #540): `die` from an impossible replayed pick under `--resume`, `die` from a
  bad `--seed`/`--guide`, a panic that is not a `FATAL_MARKER` (the default hook
  runs — then what?), SIGQUIT and SIGPIPE (no handler is installed for either),
  and the parent killed with SIGKILL. For each, record `ps -eo pid,ppid` for
  surviving stubs and the count of `/tmp/mtg-draft-claude-code-*` left behind. An
  exit that orphans a live metered call costs what #206 cost whatever caused it,
  so the question underneath is whether the guard can be one place every exit
  goes through rather than a signal handler plus three ad-hoc paths

  **Run 2026-10-03: the signal and seat-fatal paths are fixed; the
  other exits are not.** SIGINT, SIGTERM and a seat-fatal `die` swept all
  eight groups at `--players 8`. `die` from `--seed`/`--guide` comes
  before any seat exists. Leaked scratch dirs are reaped by the next
  run's `prepare_seat`. All of that is **checked, correct**.

  What still orphans or wedges:
  - SIGQUIT (#653) and SIGKILL (#654).
  - A closed stderr (#652): Rust ignores SIGPIPE, so `eprintln!` panics,
    `die`'s own first `eprintln!` panics before the sweep, and
    `REPORTED` parks every thread for ever. That is the concrete case for
    making `die` do the sweep first and its I/O infallibly.

  Two method notes:
  - Launch the runner under test from something that resets SIGQUIT to
    `SIG_DFL`. A bash `&` job inherits it ignored and SIGQUIT will look
    handled.
  - Put a unique marker in the stub's path, so
    `ps -eo pid,ppid,pgid,cmd | grep <marker>` finds only your trees
    while other probes run.

- D22 [proposed 2026-09-18, from #541, #538 and the join loop in `main.rs:861`]
  the tournament phase as a concurrency surface: every probe so far has run
  `--players 2`, where the tournament is one match on one thread and nothing is
  actually concurrent — which is exactly why D12's 2-seat pair passed and its
  4-seat pairs did not. At `--players 8 --best-of 3` a round is four `play_match`
  threads and eight simultaneous `claude -p` children, on top of the eight the
  draft already spawns. Run a full 8-seat pod under a stub and check what the
  tournament shares: does the process-group table cover the *game* backend's
  children or only the draft's; does Ctrl-C mid-round orphan four matches' worth
  of subprocesses (#538 says the table has four slots); do two matches in one
  round ever interleave a `GAME` block or a `MATCH` line (those are emitted after
  the join, so they should not — verify it, because the LLM round-trip records
  demonstrably do, #541); and does one match's fatal reach the operator or does
  the join order swallow it the way #539 describes for the pick loop. Anything a
  `play_match` worker touches that is not its own `PlayerSpec` — the registry,
  the card reference, the log mutex, the usage counters the summary adds up — is
  where to look

  **Run 2026-09-23: the concurrency is sound and the bookkeeping around it is
  not.** Nine eight-seat runs (`--best-of 3`, seeds 7 and 41 twice each) and two
  four-seat runs under a schema-walking stub, plus a SIGINT run whose game calls
  each slept 90-120s holding a `sleep` descendant on stdout, and a run with one
  seat's backend dead for the whole tournament. Five things are **checked,
  correct**, and a re-probe should not pay for them again: SIGINT with four
  matches in flight killed all four stubs *and* all four descendants with 0
  orphans, so the game backend's children are in the same `LIVE_GROUPS` and
  #538's fix covers the phase it was not written for; no `MATCH` line or `GAME`
  block interleaves, because both are written from `main` after the join; the
  usage counters survive four match threads to the call (936 stub invocations =
  344 draft + 592 game, 117 failed, printed `Games: 475 calls`); pairings never
  repeat over three rounds at eight seats; and a match's fatal is *not* swallowed
  by the join order, because the tournament workers go through `in_seat`, so
  #539 does not reach here. What broke: `buffer_here()` is installed on the
  deck-build scope (`main.rs:880`) and the tournament scope (`:994`) and **not on
  the pick loop**, so #542's `SESSION` record — written from inside the pick
  worker at `llm_client.rs:768` — is the one block that escapes #541's
  machinery, and one seed writes two logs again (#586); a seat whose subprocess
  died is reported as a model that answered `{}`, on a retry window measured at
  ≈6.5s against the draft's 600s (#587); and `standings_row` carries a bye but
  not a substituted deck, a forfeited game or 39 substituted answers (#588).
  Three method notes. **Give the stub a concede branch** — answer the action
  menu with the `Concede` row when the schema offers it — or an eight-seat
  best-of-three tournament is tens of thousands of calls; with it a full run is
  ten seconds. **A conversation id is not a seat discriminator**: a retried
  *first* call mints a fresh `--session-id` each attempt, so a stub keyed on it
  fails once and then stops. Hash the `--system-prompt` instead, which carries
  that seat's decklist and is stable across its whole match. And a determinism
  claim needs `diff <(sort a) <(sort b)` beside the plain `diff` — an empty
  sorted diff with a non-empty ordered one is the ordering defect, and is
  exactly what #586 looks like.

- D23 [proposed 2026-09-23, from #586 and the D22 night] every record a worker
  writes, not just the ones #541 knew about: #541's fix is a convention, not a
  mechanism — `game_log::buffer_here()` is called at exactly two sites
  (`main.rs:880`, `:994`) and any `write_at`/`write` reachable from a thread that
  did not call it lands in the log in scheduler order. #542's `SESSION` line is
  one such site and put the non-determinism straight back (#586); nothing says it
  is the only one. Enumerate every `mtg_player::game_log` write reachable from a
  `spawn_seat` worker — the pick loop, the deck-build scope, `play_match` and
  everything `mtg-player` writes underneath it — and for each say which of the
  two buffered scopes it is inside. Then test the enumeration rather than trust
  it: run the same `--seed` twice at `--players 8 --best-of 3` under a stub that
  is a pure function of prompt‖schema, and require **both** `diff` and
  `diff <(sort a) <(sort b)` to be empty after masking timestamps and session
  uuids. Force the seat-level paths that a clean run never reaches — a rejected
  answer, a `FALLBACK`, a `STALLED` game, an `API_RETRY`, a `MALFORMED` — so the
  error records are in the comparison too, since those are exactly the lines
  written from the deepest point in a worker. The underlying question is whether
  the buffer should be a property of the *logger* (records tagged with their
  seat and emitted in seat order at the end of a scope) rather than something
  each new call site has to remember to be inside, because the convention has now
  been broken once by a fix that had no reason to know about it

  **Run 2026-10-03: #586 is fixed, and Error records are the one class
  the buffer was designed to miss.** The pick loop now buffers
  (`main.rs:1000/1027`), and every Info/Debug write reachable from a
  worker is inside one of the three scopes. Thirteen eight-seat
  best-of-3 runs gave a sorted diff of 0 on every pair, including
  STALLED, FALLBACK, NO_ANSWER and every API_* record.

  The ordered diff is never empty, because `game_log.rs` writes Error
  records through on the premise that a clean run has none (#658). A
  schema-walking stub that ignores `minItems` gets about 25 MALFORMED
  per run, which is enough to show it. Game-side API records name no
  seat (#659).

  A re-probe after #658 is fixed should require the **ordered** diff
  empty, with forced errors in both runs. It should also check that a
  match's MALFORMED lines sit inside that match's block.

- D24 [proposed 2026-09-28, from #609's fix] does a seat play the match it is
  now told it is in? #609 gave the system prompt the game number, the score
  from the seat's own side, and what winning or losing this game settles — so
  the states that text names are now checkable at the surface. Run
  `--players 2 --best-of 4 --model cc --seed 7` under a stub and, for each
  game, read the `## This match` section against the `MATCH`/game records that
  follow it: the game number must match the games already logged, the two
  seats' scores must be mirrors, and the stake sentence must be true of the
  position. Then force the three positions the prose makes claims about and
  check the claim holds rather than that it was printed: a seat at 2-0 in a
  best-of-4 (winning takes it), a seat at 0-2 (losing loses it), and — the one
  worth the night — a seat leading **2-1 going into game 4**, where the match
  is won 2-1 at the cap without anyone reaching `wins_needed` of 3, so winning
  takes the match and losing only levels it to a draw. That last case is the
  one a threshold-only reading of `wins_needed` gets wrong, and it is reachable
  at every even `--best-of`. Worth asking of a real `cc` seat rather than a
  stub for one match, since the question is whether a seat told it is playing
  for a draw plays differently from one told it is playing for a win — a
  concession at 19 life in a game it cannot win the match with would be the
  visible tell. The engine-side truth to compare against is
  `MatchResult::winner` (more game wins, `None` if level) and `match_is_over`
  (`wins_needed` **or** `best_of` games played, #484), not `wins_needed` alone.

  **Run 2026-10-03: correct in every position without a drawn game.**
  Eight runs at best-of 1-5 under a stub that reads the game number from
  `**This is game G of at most N.**` and the seat from a hash of the
  system prompt minus `## This match`, conceding on a plan. Checked:

  - every game number;
  - the score mirror;
  - the stake sentence, including 2-1 into game 4 of a best-of-4, played
    to both 3-1 and a 2-2 Draw.

  The stake breaks only once a game is drawn: 2-0 going into the last
  game is told it decides the match (#649). A stub cannot reach a drawn
  game cheaply, so check it by calling `MatchFormat::match_section` from
  a scratch crate.

  Side finds: `(winner: Seat draw)` on stderr (#650), and a bye credited
  `wins_needed` game wins rather than 2-0 (#651). A real `cc` seat
  playing for a draw was not tried; it is the half of this idea still
  open.

- D25 [proposed 2026-10-03, from #655 and #656] the snapshot's shape, not
  just its cards: `--resume` refuses a replayed pick naming a card that is
  not in the pack, but it counts records rather than seats, so a save
  with two records for one seat replays both and exits 0 (#655), and a
  round with one seat missing is silently re-asked live (#656). Take a
  real `--save` and corrupt it every way a crash or a hand edit could —
  a seat index past `--players`, a negative or duplicated `pick`, a
  round's records out of order, a pick from pack 2 before pack 1 is
  complete, a different `--players` or set from the one it was drafted
  at, a truncated last line, a file from a newer version — and for each
  record whether the run refuses with a message naming the bad record,
  or replays something and draws a different draft in silence. Compare
  the "N picks came from a snapshot" summary against the number actually
  replayed every time.

  **Run 2026-10-08: the shape check holds, but not what happens around it.**
  About 30 corruptions of a 2-seat `--seed 25` save were tried. Every one
  was refused with the record named, or replayed the identical draft:
  seat past the pod, pick -1/0/15, round 4, duplicates, shuffled order,
  pack 2 begun early, edited players/seed/set, truncated or empty files.
  Four finds, all outside the check:

  - #733: the replay writes `--save` before the impossible-card check
    reaches the bad step, so a refused `--resume X --save X` truncates its
    own snapshot.
  - #732: a resumed run's save drops every carried match.
  - #734: `players: 0` panics.
  - #735: a refused resume's log claims the picks were replayed and holds
    no error.

  Accepted silently but harmless today: unknown fields, `version: 99`, a
  `seats` list of the wrong length. The save has no version field, so a
  future format change will not be caught. Re-probe: whether *every*
  `die` after the first snapshot write leaves `--save` intact (the card
  check was one; look for others).

- D26 [proposed 2026-10-08, from #581 and reading `DraftSave` during D25]
  the tournament half of the snapshot. #581 added `decks` and `matches` to
  the save. `check_snapshot_shape` reads only `picks`, and the save does
  not record `best_of`. Take a finished `--players 4 --best-of 1` save made
  under a stub that answers every deck call with `{}`. The runner then
  substitutes the decks, so a run takes about 40 s from the repo root.
  Cut, edit and re-resume it:
  - a different `--best-of`, including leaving it off (the default is 3);
  - a deck under 40 cards, or with cards from another seat's pool, or a
    name that is not a card;
  - a match record with an impossible score, with its seats swapped, for a
    round that never happens, or duplicated;
  - a finished save resumed with `--save` to a new path. Count the matches
    in the new file.

  For each, check whether the run refuses with the record named, the way
  picks are refused, or plays on.

  **Run 2026-10-08: nothing past the picks is checked.**
  - #729: best-of is taken from the flags, so formats mix.
  - #730: decks are played unvalidated, and an unknown card panics a
    match worker.
  - #731: impossible scores are carried, and unmatched records are dropped
    and the match re-played, both in silence.
  - #732: the new save from a resume holds no matches.

  Still open: what the *game* half of a resumed match does. A match is
  saved only when it finishes, so an interrupted best-of-3 at 1-0
  re-plays game 1. Check whether the log and the stake prose (#609) say so.
- D27 [proposed 2026-10-08, from docs/plans/draft-with-friends.md] the
  table with a person at it: host `mtg-draft-server --seats human,ai,ai,ai`
  under a `CLAUDE_CODE_BIN` stub, join seat 0 from the Playwright Chromium
  (the page) and, on a second run, from `mtg-draft-client`, and draft to
  the end. Verify against the log: every pick the page showed is the
  pick the log records, the pass direction flips at pack 2, a pack that
  arrives while you are still holding one is counted ("1 pack waiting")
  and arrives in order, pick numbering on a pack is `size - remaining +
  1` whoever held it before you, and the view never carries another
  seat's pack or pool (read the raw `view` JSON from `/api/view`, not
  just the screen)
- D28 [proposed 2026-10-08, same source] the absent human: a human seat
  that never joins, and one that joins and stops picking. Without
  `--pick-seconds` the table must wait and say for whom; with it, the
  auto-pick must be logged as `auto-pick: <why>`, shown to the seat as a
  notice when it returns, and never name a card the pack did not hold.
  `kick <seat>` at the host's keyboard turns the seat into an auto-picker
  for the rest of the run — check the deck it ends up with is buildable
  and its matches are recorded as forfeits, not as games nobody played
- D29 [proposed 2026-10-08, same source] two people, one key: open the
  same seat's URL in two tabs and pick from both. One pick must win and
  the other be refused against the `pack_id` it named, both tabs then
  showing the same next pack; a reconnecting tab is sent the pending
  pack. Then a wrong key, a seat index past the pod, and a join after the
  draft is over — each refused with a reason, none taking the server
  down
- D30 [proposed 2026-10-08, same source] the human deck: build a deck of
  39, of 40 with a card not in the pool, of 40 with 17 basics, re-send a
  deck after `ready`. The server's answer must match
  `mtg_draft::deckbuilding::validate_deck`, the page's Ready button must
  have been disabled for the same reason the server would refuse, and
  the `decks/seat-N.txt` file written beside the log must be one
  `mtg-runner --deck1` loads and plays
- D31 [proposed 2026-10-08, same source] the human's match: from a
  finished 2-human draft, open each seat's game URL in its own Chromium
  page and play the match out with `autoplay.js`-style random clicks.
  Both ports must be on the `--bind` address, both pages must see the
  same game, the result must reach the draft page's standings and the
  log, and the next round must pair on it. Then a human against an AI
  seat: the AI seat's game must run on `cc`, never a metered API
- D32 [proposed 2026-10-08, same source] what the AI seats are told at a
  mixed table: read the stub's recorded prompts from a mixed-seat run
  against the runner's for the same seed and pod size. Nothing in the
  prompt may say which seats are human, and every fact the runner's
  prompt carries (pack, pool, pool shape, seat, pass direction, the
  seat's notes from its last pick) must be there — the table builds its
  prompts through the same function, and this checks it stayed that way
- D33 [from reading `build_deadline`/`auto_build` in `lobby.rs`] the build
  clock: host with `--build-seconds N` and let it run out with the seat
  in each state — untouched, a short deck sent, a legal 40 sent without
  Ready, a 40 edited card by card, a 40 sent then the tab closed, Ready
  just before the deadline. Scripted WebSocket seats put six states on
  one 8-seat table. Verify the deck played (`decks/seat-N.txt`) is the one
  the person built where there was one, the log's reason is true, and the
  page and the client show the clock and then what happened. Also try
  `0` and u64::MAX for both clocks.

  **Run 2026-10-13: the clock throws the person's deck away.** Only Ready
  survives it. Every other sent deck became `fallback_deck`, logged "no
  deck was submitted" (#810). An early finisher builds on the clock under
  "Waiting for a pack" (#811). u64::MAX panics and wedges the table (#812).
  The client shows no clock (#813) and a stale deck (#815). The page's
  notice lasts ~1 ms (#817).
- D34 [from `next_round`'s `log_bye!` and the even-only D27-D31 tables]
  the bye with a person at it: host 3- and 5-seat tables with a human at
  the bye in round 1 and again in a later round, page and client both, and
  `mtg-draft-runner --players 3` on the same seed beside it. Verify the
  Swiss bye rule, the bye's score on every surface, the pass direction and
  pick numbering with an odd pod, and what the seat at the bye is told.

  **Run 2026-10-13: paired and scored right, equal to the runner. The
  person is barely told.** The bye seat sees "No pairings yet", and the
  client says nothing (#814). The terminal and client use seat numbers
  once the tournament starts (#816). "done" beside 0-0 standings (#818).
  Event order (#819). Log threads (#820).
- D35 [from `play_one`/`forfeit_by` in `server.rs` and two D28 gaps] a
  person leaves mid-match: close the game tab, close the draft tab, reopen
  the game URL, `kick` mid-match and before the round, the opponent
  concedes while the seat is away, SIGINT and `quit` mid-match, on
  `human,human` and `human,ai` tables. Verify the match ends or waits and
  says which on every surface, a forfeit is recorded as one with a
  reason, the next round pairs, and nothing is orphaned.

  **Run 2026-10-13: a kick mid-match never ends the match**, so the
  table cannot finish (#821). SIGINT leaves no line (#823). A closed game
  page reads "OPPONENT IS THINKING" and "here" (#822). `quit` resets
  sockets unannounced (#824). A kick is unlogged (#825). Text (#826).
  After a fix for #821, re-run the kick and the concede-while-away case.
- D36 [proposed 2026-10-13, from D33/#811] the uneven finish: host
  `--seats human,human,ai,ai --pick-seconds 30 --build-seconds 60` behind
  a stub that sleeps 5-10 s a pick, so the people finish drafting first.
  Build and send a deck in the gap while the phase is still `drafting`, on
  the page and the client. Verify both show the deck screen and the clock
  from the last pick, the deck sent in the gap is the one played, the
  timer starts when building is possible, and DECK BUILDING in the log
  stays readable with the last picks interleaved.

  **Run 2026-10-18: the gap is handled.** Page and client switch to the
  deck screen with a full clock at their own last pick. A deck made
  Ready, sent but not Ready, or short (topped up) in the gap is the one
  played. Found: a client reconnect prints "X is now called X" (#856); the
  timer's auto-pick prints after "drafted all" (on #819). To open a real
  gap, slow only the AIs' Pack 3 Pick 13/14 in the stub. 5-10 s on every
  pick gives about 3 s, because a seat's last pick waits on its upstream
  seat's 13th.
- D37 [proposed 2026-10-13, from D34/#818] mid-round truth: on a 5- or
  7-seat table hold one human's game open while the AI matches and the
  bye finish. Snapshot the page, the client, `/api/view` and the host
  terminal at paired, one match done, all but one done, and round over.
  Verify pairings, standings, seat statuses and the waiting line agree at
  every step, and the bye seat and an early finisher are each told what
  they are waiting for.
- D38 [proposed 2026-10-13, from D35/#821/#822] a seat that leaves between
  games: `--seats human,human,ai,ai --best-of 3`. Play game 1 out, close
  one human's game tab before game 2 is dealt, and separately kill a
  terminal client at game 2's mulligan. Verify whether game 2 is dealt to
  an empty page, what the opponent's game and draft pages say, whether
  `status` names the seat the table waits on, and that a reconnect partway
  through game 2 resumes at the right game number and score.

  **Run 2026-10-18, with D35's kick re-run.** #821 holds only when the
  kicked seat is the one being asked. A kick while the opponent holds
  the decision leaves the game running, and the kicked seat can win it
  (#854). Game-page ports are never freed, so `--game-ports 8821-8822`
  forfeits round 2 (#853). The client never shows the score (#855). A
  game dealt to a closed tab reads "here" everywhere (on #822).
  Reconnects resume at the right game.
- D39 [proposed 2026-10-13, from R1/#809 and #808] every answer sent
  twice: at a table with packs waiting, drive `mtg-draft-client` through a
  timestamping wrapper and send each answer twice 50-300 ms apart — a pick
  number, `add N`/`drop N`, `ready`. The page's equivalents are a
  double-click and a held Enter. Verify a second answer never lands on a
  pack or state printed less than ~300 ms earlier (the page's #806 guard,
  the CLI's #71). While there, feed one cost per colour (W/U/B/R/G/C)
  through the game page's `costText`, its pool strip and both pages' pips,
  and check every letter.

  **Run 2026-10-18: picks are safe twice, on the client and the page.**
  Two things are not. The client exits on any refusal containing "key"
  or "seat" (`lands turkey=1`, #851). The page's build-row toggle has no
  repeat guard (#852). Sort a surface's commands first into "sending it
  again changes nothing" and "it toggles": only the second needs a guard.
  Every colour letter is right; ISD has no hybrid mana.
- D40 [proposed 2026-10-18, from #853 and #854] what a round leaves
  behind: host `--seats human,human,human,human --best-of 3` with
  `--game-ports` exactly as wide as one round needs, and play three
  rounds. After each match, list the server's listening ports, its
  threads (`ls /proc/<pid>/task` with their `comm`) and its stub
  children. Open the finished match's game URL again. Then do every host
  action (`kick`, `status`, `quit`) once while the target seat is being
  asked and once while its opponent is. Verify every match frees what it
  took, a finished URL says the match is over, and each action does the
  same thing at both moments.
- D41 [proposed 2026-10-20, from the draft cost work, `reports/llm-cost.md`
  phase 4] the pick nobody is asked: a pack's last card is now taken without
  a model call, in the runner (`main.rs`, the worker's early return) and at
  the hosted table (`server.rs`, `ai_worker`). Under the prompt stub, run
  `--players 2 --best-of 1 --model cc --seed <n>` and a hosted table with
  two `ai` seats and one human; count the calls (`STUB_CALLS`) and the
  log's `PICK` lines. Verify every seat is asked 39 picks, never the
  fourteenth of a pack; that the fourteenth is logged as taken without
  asking and still lands in the pool, the snapshot and the page's pick
  list; that the seat's notes from pick 13 still open its next pack's
  first prompt; and that `--resume` from a snapshot cut on either side of
  a fourteenth pick replays it without a call. Then the human: does the
  page still show a one-card pack, and does the timer still auto-pick it?
- D42 [proposed 2026-10-20, same source] kept picks are the eval's ground
  truth: `MTG_DRAFT_POSITIONS` writes one line per asked pick, and
  `mtg-draft-replay` asks it again. Under the stub, keep a draft's picks,
  then a `--resume`d draft's, then a draft where one seat's answers are
  all unusable (a stub that answers out of range). Verify the file holds
  each asked pick once — a replayed pick is not kept again, a substituted
  one says `substituted` — that every line's `notes` is what the seat was
  shown at that pick, and that `mtg-draft-replay --model cc` under the
  stub chooses `live` on every line (`kept_picks_replay.rs` checks one
  clean draft; the resumed and the substituted ones are unchecked). A
  metered `--model` must be refused before anything is read.
- D43 [proposed 2026-10-21, from the sideboarding change (`lobby.rs`
  `open_between`/`take_between`, `server.rs` `between_games`, `game.rs`
  `play_match`)] the time between games, at the hosted table: host
  `--seats human,human,ai,ai --best-of 3 --sideboard-seconds 120` and play
  both humans, one from the draft page and one from `mtg-draft-client`.
  Before game 2, try every way to send a deck the server must refuse (39
  cards, a card from the other seat's pool, a non-basic land, a sideboard
  after `ready`) and one it must take; move a card out and back in and
  check the host's terminal does not say the seat sideboarded. Verify game
  2 is played with the deck the server last took (read the game log's
  decklist), that the chooser is the game-1 loser, and that the clock ends
  it with the decks as they stand. Then the runner: under the prompt stub
  (which sideboards half the time) read every `SIDEBOARD before game`
  line against the deck the next game's log plays
- D44 [proposed 2026-10-21, same source, MTR 2.2] who plays first. Over a
  dozen stub matches at `--best-of 3`, read each game's first turn against
  the log's `chose to play first`/`draws first`: game 1 the coin flip's
  winner chooses, later games the previous game's loser, and after a
  DRAWN game whoever chose before (drive one to a draw with the action
  budget). Check the game seat's system prompt ("The loser of game N
  chose who goes first") and the game page's match line say the same
  thing the engine did, and that a person who never chooses plays first
- D45 [proposed 2026-10-21, from the tiebreakers (`tournament.rs`
  `sorted_standings`, MTR Appendix C)] standings by hand: host or run a
  six- or seven-seat table with a bye and, if you can force one, a drawn
  match. After every round compute OMW%, GW% and OGW% by hand (the 33%
  floor applies to each opponent's own percentage; a bye is no opponent)
  and compare the runner's FINAL STANDINGS, the page's standings column,
  `mtg-draft-client`'s line and `/api/view`'s `omw`/`gw`/`ogw`. Before
  round 1 counts no surface may show a tiebreaker at all
- D46 [proposed 2026-10-21, from the hosted table's snapshot (`lobby.rs`
  `snapshot`/`restore`, `--resume`)] stop and carry on: stop a hosted
  table with Ctrl-C mid-draft, mid-build (one deck ready, one not),
  between games of a match, and mid-game; `--resume` each from
  `table.save.json`. Verify the join links still work, every seat's
  pool, picks, name and AI notes are what they were, a ready deck is
  still ready, and a match in progress is played again from game 1 with
  nothing of the stopped game in the standings. Then edit the snapshot
  (a pick of a card the pack never had, a deck the pool cannot build) and
  check each is refused with its reason, before anything is dealt
- D47 [proposed 2026-10-21, from #822-#825] where everybody is: with a
  person in a match, close their game page, then their draft page, then
  reopen each; kick them mid-game; Ctrl-C the server. At every step read
  the other person's draft page and game page, the host's `status`, and
  `mtg-draft-client`. Verify the presence word is the same everywhere
  ("in game", "game page closed", "away", "kicked"), the opponent's game
  page says whose page is closed instead of "Opponent is thinking", and
  every page and client says the table is closed and why, and stops
  reconnecting
- D48 [proposed 2026-10-21, from `scripts/draft-loop/`] the nightly loop's
  own numbers: run `DRAFT_LOOP_OUT=<scratch> DRAFT_LOOP_PLAYERS=4
  DRAFT_LOOP_BEST_OF=3 CLAUDE_CODE_BIN=$PWD/scripts/llm-prompt-stub.py
  scripts/draft-loop/night.sh 2099-01-01` and check `nights/*.json` against
  the run's own log: every seat's 42 picks in order, its deck, its match
  and game record (the log's MATCH lines), which seats had the guide.
  Then read a REAL night in `reports/draft-loop/nights/`: does a seat's
  lesson name a card it never drafted or faced, does `stats.md` count a
  card's games right for a deck that sideboarded it out, and does the
  guide name a card the statistics and lessons do not?
- D49 [proposed 2026-10-21, from the one-port hosting (`server.rs`
  `proxy_game_page`)] one port, nothing else: host `--bind 0.0.0.0` with a
  human seat and open the match link from another machine (or another
  network namespace) with only `--port` reachable. Play a whole game
  through it, reload the page mid-game, and close and reopen it. Then try
  to reach anything else through the lobby: `/game/<the lobby's port>/`,
  `/game/22/`, a finished match's port, `/game/<live port>/../../`,
  `/game/<live port>/assets/../../Cargo.toml`, a request line with no path,
  and a WebSocket upgrade to a port that is not a game page. Verify each is
  refused without the lobby opening a connection anywhere (watch with `ss
  -tnp`), and that `netstat` shows the game pages listening on loopback
  only. `--direct-game-ports` must still hand out per-port links
- D50 [proposed 2026-10-21, from the host's `cli` seat (`server.rs`
  `run`/`host_loop`, `client.rs`)] the host at the keyboard: host
  `--seats cli,human,ai,ai` (with and without `--quiet`) and play the cli
  seat from the server's terminal while a person plays from the page.
  Type every client command and every host command at each phase,
  including the words both could mean (`help`, `quit`, an empty line, a
  number while the host is not picking, `start` after the draft started),
  and `kick 0` on the host's own seat. Verify each line goes where the
  docs say, that the table's event lines never hide the seat's pack, that
  the seat's game link works through the lobby's port, and that `--resume`
  of a table with a cli seat puts the host back at the keyboard
