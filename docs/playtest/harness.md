# Playtesting the harness

Subject: the LLM interface — the prompts, the response schema and the
conversation an LLM seat plays a game through. Not the CLI a human
drives, and not the rules: this is about whether a model sitting in a
seat is told what it needs, offered what it may do, and understood when
it answers.

This is the newest subject and the thinnest. Almost nothing here has
been played.

It is also the surface that fails most quietly. The CLI tells you when it
is broken — a clipped row is visible, a wedged prompt is visible. Here a
rejected schema, a prompt too long to read and a seat that answered
nothing all look the same from outside: the game plays on. See "One
decision, three surfaces" in `README.md`; several ideas below are the
harness half of a CLI idea, and are worth running on the same night.

## Before you start

The seat is `--p1 claude-code` (`cc`), which runs `claude -p` on the
CLI's own login. **Never** `--p1 claude` or `--p1 gemini` — those are
metered API calls and are forbidden outright.

The ideas below are a starting point, not a syllabus. They are what
previous nights happened to think of, and the bugs that mattered most
were usually not on the list when the night began. The real method is the
one underneath them: read the code that implements this, read the rule or
the contract it is supposed to satisfy, and find where the two disagree.
When you find a way to look that the list doesn't have, take it — and
then add it, per "Adding an idea" in `docs/playtest/README.md`.

## Where to look

- `mtg-player/src/llm.rs` is the whole harness: `GAME_RULES` (the prompt
  format contract in the system prompt), `format_turn_header`,
  `format_state_body`, `format_perms_compact`, `build_prompt`, the
  per-prompt action formatters, and the response parsers. The backends
  are in `mtg-player/src/llm/`.
- `docs/llm-harness.md` for how each seat is invoked and what it costs.
- Two contracts to test against, and they are different questions.
  *Correct*: does what the seat is told match the game state, do the
  offered actions match what the engine will accept, is hidden
  information absent? *Sufficient*: could you make this decision well
  from the prompt text alone? A prompt can be perfectly accurate about
  everything it mentions and still omit the thing that decides the game.
- `CLAUDE_CODE_BIN` points the seat at any executable, which makes the
  harness's failure paths reachable: a wrapper that logs argv and stdin,
  or a script that returns whatever you want the model to have said.

## Ideas

- H1 prompt protocol: run `--p1 cc --p2 random` with `--log` and read what
  the LLM seat is told against what the game state actually is — every
  field of the prompt format in `mtg-player/src/llm.rs` documented and
  populated, hidden information never in the seat's prompt, the legal
  actions listed matching what the engine will accept, and no
  schema-valid answer that the engine then rejects
- H2 subprocess contract: point `CLAUDE_CODE_BIN` at a wrapper that logs
  argv and stdin and delegates to the real binary — session ids stable
  across a game, one subprocess per decision, no leaked processes or temp
  directories after exit, Ctrl-C or kill, the game never blocking past
  the call timeout, and end-of-game usage totals that make sense
- H3 harness failure modes: point `CLAUDE_CODE_BIN` at a script that
  exits non-zero, hangs, prints invalid JSON, prints a well-formed but
  illegal action, or answers the previous prompt. Every one must be a
  clean recovery or a clean failure; none may cause the game to take an
  action the seat did not choose
- H5 [proposed 2026-09-04, from reading `format_state_body` and
  `format_perms_compact` in `mtg-player/src/llm.rs`] information
  sufficiency: H1 asks whether what the seat is told is *correct*; this
  asks whether it is *enough*. Play a `cc` seat with `--log` and at every
  decision try to make the choice from the prompt text alone, then look at
  the real game state for what you needed and didn't have. Suspects to
  confirm or clear: exile is a count with no contents; land lines carry a
  name and tapped state but no rules text, so a utility land's activated
  ability is invisible; creature lines carry keywords but no rules text,
  and the "Card reference" block covers only the seat's own decklist, so
  an opponent's creature may have no text anywhere in the prompt; nothing
  states what mana is actually available. File a gap when it would change
  a decision, not for every omission
- H4 recap fidelity across resume: `--save` and `--resume` a game with a
  `cc` seat and check the conversation the resumed seat is handed
  describes the same game it left — recap contents, turn count, nothing
  hallucinated and nothing dropped
- H6 [proposed 2026-09-04, from #203] the stdout-holder: point
  `CLAUDE_CODE_BIN` at a script that writes a valid answer and then leaves a
  background child holding stdout open, and at one that does the same after
  exiting normally. #203 found the call finishing only on EOF rather than on
  the watchdog's kill, in the hang case; confirm whether that is the general
  case — a well-behaved seat with one stray grandchild should not wedge a
  game, and the timeout should end the call whatever else holds the pipe
  — **played 2026-09-10: the claim generalizes, and it is not about
  hanging.** #203's kill works (the never-answering holder times out on
  schedule, kills the group, logs `exhausted all 3 attempts`, plays on) and
  #206's process/workdir leak is gone on every path but one. But the call
  still completes on **EOF**: `call_once`'s comment says it waits on the
  result, and the reader thread's `tx.send` happens only after
  `read_to_string` returns. So a well-behaved seat — valid answer, exit 0 —
  with one stray grandchild pays that grandchild's lifetime on *every*
  decision (0.88s to 87.87s over 29 calls, with the `claude -p` child
  already a zombie), and a grandchild outliving `CALL_TIMEOUT` makes the
  harness throw away 29 complete correct answers and play the game mute
  (#458). The timeout also guards stdout only: `stderr_reader.join()` has no
  deadline, so a grandchild holding **stderr** reproduces #203 in full —
  wedged with the answer already read, empty log, zero `API_ERROR` — and
  `drop(group)` runs before that blocking join, so Ctrl-C orphans the tree
  (#459). Next time, probe whichever pipe a fix does *not* cover, and
  measure wall clock per decision rather than watching for a wedge: the
  expensive failure here looked like a working game
- H7 [proposed 2026-09-04, from #209] display-index vs legal-index audit:
  #209 found `pick_action_index` given an index into the *displayed* option
  list while testing its concede guard against `legal_actions`, so the guard
  vanishes whenever duplicate permanents collapse two options into one. Walk
  every call site (`mtg-player/src/llm.rs` lines 1889/2014/2045/2077 pass
  `&[]`) against its own display list, and force a collapsed list at each —
  any other guard or lookup keyed off the wrong list is the same bug
- H8 [proposed 2026-09-09, from #398] the response schema is checked by the
  API before the model ever sees it: top-level property keys must match
  `^[a-zA-Z0-9_.-]{1,64}$`, and a key that fails is a 400 — no tokens, no
  answer, and a harness that turns the failure into `{}`, which reads
  exactly like a seat declining. #398 was three prompts keyed by card
  display name; a `cc` seat could not cast Skaab Goliath at all, six
  attempts in one game, and nothing in the game log said why. Walk every
  `send_message_structured` call site and check its schema against the
  pattern — the isolated two-command repro is in #398 and needs no game.
  Then check the OTHER half, which no test can: that a schema the API
  accepts is one the model can actually answer. `enum` lists of a hundred
  indices, `minItems`/`maxItems` that contradict the prompt text, a
  `required` field the prompt never explains. And check what the harness
  does with a refusal: an empty answer must be distinguishable from a
  chosen "none", or a seat that never got the question looks like a seat
  that passed
  — **played 2026-09-10: the schema half is clean and the refusal half is
  where everything is.** A `CLAUDE_CODE_BIN` stub that records every
  `(prompt, schema)` pair harvested 4,311 structured requests over 38
  games and reached 9 distinct top-level shapes with **zero** keys failing
  the pattern; both request paths are literal-keyed (the draft's only
  dynamic keys nest under `maindeck`). Six direct `claude -p --json-schema`
  calls then settled the half no test can reach: every shape the harness
  emits is accepted *and* answered — a 253-value integer enum (Nevermore
  offers every implemented nonland card name, the largest enum in the
  program), nested card-name keys with spaces and `#`, integers as string
  enums, `"properties": {}` objects inside `required`,
  `minItems`/`maxItems` on an enum array — while the control, those same
  keys at the top level, still returns the #398 400 verbatim. What broke is
  the third part: `send_with_schema` substitutes `{}`, and of the ten
  callers only four (`pick_action_index`, the mulligan, `choose_card_set`,
  `choose_x_funding`) log and count it. `mark_indices`, the blockers and
  attackers prompts, `choose_pile_division`, `choose_ordering` and
  `confirm_concede` write a line indistinguishable from a decision, or
  nothing — measured at 4 mute target-set prompts with 0 `MALFORMED` lines
  and a summary reporting 0 rejections (comment on #399). Note also that a
  *successful* call with no parseable `structured_output` produces no
  `API_ERROR` line either, so a remedy keyed on retry exhaustion misses a
  refusal, a truncation and an answer to the wrong schema alike. Two
  defects came out of it: #462 (nothing bounds the cast-cancel-recast cycle
  the empty set produces) and #466 (the system prompt carries the
  opponent's whole decklist). Worth knowing for next time: the rule's only
  runtime enforcement is a `debug_assert!` compiled out of the release
  binary, and `llm_request_shape.rs` asserts the predicate rather than the
  schemas, so the harvest is the enforcement
- H9 [proposed 2026-09-09, from V7/V42 and `format_action_prompt`] the
  prompt as a thing with a SIZE. The CLI wraps, pages and clips; the LLM
  prompt does none of that — `format_action_prompt` joins every legal
  action into one comma-separated line with no cap, and the board state,
  the card reference and the log all grow without one. Play a `cc` seat
  into the widest boards V42 builds and read what it is handed: how long
  is the actions line at 60 legal actions, how much of the prompt is the
  recap, and is the thing being decided still findable in it. A prompt a
  person would call unreadable is the model's whole input. File a gap when
  the decision is buried, not for length alone
  — **played 2026-09-10: the decision is NOT buried, and the biggest
  section is not one of the four named above.** The question is last and
  clearly delimited in all 3,471 captured prompts (including a 442-line
  one), and ability rows carry both `(#id)` and the ability text, so two
  identical creatures are tellable apart. The dominant section is the
  **system prompt, re-sent verbatim on every call** — 28k to 69k chars,
  7k-17k tokens, 78-97% of all input characters in every run — so measure
  that first. The actions line is linear at ~111 chars per row and uncapped:
  570 chars at 10 actions, 5,450 at 60, 8,898 at 91, all on one unwrapped
  line, because activated abilities are the one row class nothing collapses
  (#461). Growth over one game was 5.0x chars and 4.1x lines from turn 3 to
  turn 24, with graveyards ungrouped (one line per card, while the *board*
  groups lands as `9x Island`) and `Recent events` neither capped nor a true
  since-last-decision delta, because only `build_prompt` advances
  `last_log_index` (#464 — 307 lines covering turns 1-98, 74% of one
  prompt). And the two places information is actually *lost* were found by
  looking for size and turned out to be the opposite: the mana-ability label
  drops the colour, so a dual land is two byte-identical rows (#460, the
  #118 fix never crossing to this surface), and the cleanup discard — the
  *smallest* prompt in the game at 345 chars — is the only decision the seat
  makes with no board, no life totals and no graveyard (#463). Method: a
  stub answering with the schema maximum concedes on its first priority
  window, since `Concede` is the highest `action` enum index, so wide boards
  need a policy that reads the labels (prefer `Play`/`Cast`, never
  `Concede`), an opponent that cannot interact (`60 Island`) and a
  self-sustaining defender
- H10 [proposed 2026-09-09, from the random seat's "up to N" answer] the
  seat that answers with a constant. Not the LLM seat: `mtg-player/src/random.rs`
  is what the invariant fuzzer plays, so a prompt it answers with a legal
  no-op is a prompt the fuzzer never really exercises — and nothing fails,
  which is what makes it worth a night. The "up to N" target slot had
  `min` of zero and the seat took the minimum, so 178 casts of Feeling of
  Dread across eight seeded games named no target and tapped nothing.
  Read every `resolution_prompt` arm in `random.rs`, ask what it answers
  when the prompt allows nothing, and then measure: run seeded games with
  a deck built for that prompt and count how often the effect actually
  DOES something in the `--log`. A seat that always answers the same way
  is a hole in the oracle whatever the tests say
  — **played 2026-09-10: its own motivating bug is fixed and the identical
  mistake was two blocks above it.** `ChooseTargetSet` rolls now and Feeling
  of Dread genuinely names 0, 1 and 2 targets; `ChooseObjectSet`, pile
  division, both combat prompts and the twelve enumerated prompts via the
  generic uniform pick all roll too — six of nine arms cleared. But the
  exile-cost arm takes `min`, which is 0 for Harvest Pyre, with a comment
  that says so: 191 casts across 20 seeded games exiled nothing and dealt no
  damage, one bucket in the whole distribution (#455). The generalisation
  worth keeping is that **the constant need not be the minimum, and need not
  be an answer at all**: "always keep" silences the entire London mulligan
  (140 decisions, 140 keeps, 0 mulls, 0 bottom lines — #456), and a seat
  that always answers with a *set* and never with `CancelCast` makes four
  engine un-stash branches, each one a past bug fix (#123, #262, #290),
  unreachable to the fuzzer forever (#457). So read a non-interactive seat
  by asking not only "is this answer a no-op" but "**which kinds of answer
  does this seat never give**" — a branch the fuzzer cannot reach is as
  quiet as an effect that does nothing. `ChooseXFunding` answers with the
  maximum, which still sweeps X=0..9 across varying boards, so it is cleared
  on coverage; the residual is that for a *given* board the intermediate X
  is never chosen and the seat always empties its mana
- H11 [proposed 2026-09-10, from #462] forward progress as a property.
  Nothing in the engine, `mtg-player` or `mtg-runner` guarantees that a game
  advances: #462 found the cast-cancel-recast cycle unbounded, 2,180
  cancelled casts in 60 seconds, turn 15 forever, ended only by `timeout`.
  Ask the same question of every prompt that can refuse a commitment.
  Method: for each structured prompt kind, point `CLAUDE_CODE_BIN` at a seat
  whose answer there is unusable *and stable* (a successful call with no
  `structured_output` is the cheapest shape) while it answers everything
  else legally, give the run a wall-clock budget, and record whether the game
  ends. Candidates beyond the exile cost: a blocker assignment the engine
  refuses for menace, an X funding of 0 on a spell needing X>=1, an ordering
  that is not a permutation, a pile division the opponent then declines. A
  prompt that can livelock is a class of game `--check-invariants` will never
  fail on, so the only way to find it is to try to hang the program
- H12 [proposed 2026-09-10, from #460 and #463] the action label and the
  prompt body as a three-surface diff. `format_single_action` in
  `mtg-player/src/llm.rs` and the `action_label` match in
  `mtg-player/src/cli.rs:3107` render the same `Action` variants for two
  different readers, and they have already drifted: the CLI names the mana
  an ability produces (the #118 fix) and the LLM label is still
  `Tap <name>`, so a dual land is two byte-identical rows and the seat
  cannot choose a colour (#460). Walk the two tables variant by variant and
  list every field one names that the other drops. Then do the same for the
  prompt *body*: which callers build their message themselves instead of
  through `build_prompt`? `choose_card_set` is one, and its discard arm is
  the only decision in the game made with no board (#463). Each difference
  is either a deliberate token saving or a decision the seat cannot make;
  say which, and cite the CLI line that proves the data was available
- H13 [proposed 2026-09-10, from H8's harvest] the prompt kinds nothing
  reaches. `send_message_structured` has ten callers; 4,311 requests
  harvested from 38 games over fourteen decks reached nine shapes and never
  `choose_pile_division` — which is the one site
  `mtg-player/tests/llm_request_shape.rs` singles out as safe *because* it
  nests its card-name keys, and whose only runtime check is a
  `debug_assert!` compiled out of the release binary. Build the board each
  unreached prompt needs (Liliana of the Veil at −6 for the pile division,
  then the opponent's `ChoosePile` answer) and put a seat through it for
  real. Keep the instrument while you are there: a stub that records every
  `(prompt, schema)` pair and validates the schema the way the API does
  audits all ten at once for the cost of one game, and the harvest is
  currently the only thing enforcing the #398 rule in a release build
