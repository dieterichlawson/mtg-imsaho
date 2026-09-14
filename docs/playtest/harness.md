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
  and the system prompt's "Card reference" covers only the seat's own
  decklist (since #466 — it was the union of both decks, which leaked the
  opponent's list), so an opponent's card has text only through the
  per-decision `Opp's cards in view:` section, which lists cards on the
  battlefield, the stack, in graveyards, in exile and revealed — check a
  card that matters and is in none of those; nothing states what mana is
  actually available. File a gap when it would change a decision, not for
  every omission
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
  — **played 2026-09-14: every index in the program is faithful, and the bug
  is one level up — the prompt those indices are chosen from.** All five
  `pick_action_index` call sites consume the list they displayed, and the
  collapse is provably index-faithful: over 11 stubbed games (1,038 structured
  requests, 9 top-level shapes, 0 illegal schema keys) **145 of 145**
  activations named the copy its row promised, including 32 that landed inside
  an `ActionRow::Copies` range and 32 at its *far* end — a stub that picks the
  highest index of the best-ranked row is how you exercise that, since the
  obvious "lowest index" policy only ever tests member 0. #209's guard is
  intact too: with the display list collapsed, 68 of 68 priority menus asked
  `confirm_concede`, 68 cancels produced `PassPriority`, and the game ended by
  decking on turn 68 with zero concessions. The other index-vs-list suspects
  all cleared by construction and are not worth re-walking — `choose_card_set`
  displays `view.your_hand` and indexes `prompt.options`, but both are
  `state.objects_in_zone(Zone::Hand, player)` in that order; the attackers and
  blockers prompts build their enums and apply their answers to the same
  `eligible`/`attackers` slices; `choose_x_funding`'s card-name keys are bucket
  keys and so unique by construction, and nested; and the engine already
  excludes a spell from its own target list (`targeting.rs:685`,
  `filter(|&id| id != spell_id)`).
  What the audit actually found is that four prompts still build their message
  with a bare `format!` and never reach `build_prompt`, so #463's "this was the
  one prompt built from a format string alone" is false:
  `prompt_target_selection`, both sacrifice prompts and the ability-target
  prompt are 59 of 417 requests, the smallest is **48 characters**
  (`Geistflame: select a target:` / `0: opponent, 1: you` — a damage spell
  aimed at a player with neither life total present, and the seat pointed it at
  itself 8 times to 5), and the CLI renders the whole board at the same decision
  *because* #122 established a human needs it (#491). The method that found it:
  diff `GAME_RULES` against the schemas actually emitted rather than reading
  either alone. That also turned up the system prompt telling every seat the
  exile-from-graveyard prompt answers with "a boolean per card" when the schema
  is an `indices` array — `mark_indices`' own doc comment says the opposite of
  the const, three functions away (#492) — and five worked examples still
  showing the comma-joined action list (#493). Nothing checks that const
  against its own formatters; `omitted_events_marker` (`llm.rs:2755`) is the one
  place that does
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
  — **played 2026-09-14: the game always ends; the DECISION is what nothing
  bounds.** Nine prompt kinds muted one at a time (a successful call with no
  `structured_output`) over ~490 games: every arm exited cleanly, none hit its
  wall-clock budget. Eight of the ten callers substitute something legal and
  cheap — no attackers, no blocks, keep, the first `min` cards, the listed
  order, X=0, all in pile 2, index 0 — so the engine is never asked to refuse;
  `choose_object_set` fills its own shortfall from the options, which matters
  because `ChooseObjectSet` is the one refusal the engine answers by *re-asking*
  rather than by cancelling a cast; and #462's residue, the `mark_indices`
  shortfall, now trips the watchdog at 100 identical decisions and dies with a
  report naming the seat and the question (ur/bg seed 11, exit 1, 4.7s, turn
  24). Two candidates the idea named are vacuous: pile division was never
  reached at all (confirming H13), and no card in the pool demands X >= 1.
  What broke is the axis nobody was counting. `choose_blockers_structured`
  validates a menace-illegal block client-side and re-sends the same prompt with
  the same schema — which still offers the illegal index — twenty times, then
  declares no blocks and throws away the legal blocks
  `mtg-engine/src/combat.rs:135-147` would have kept from the identical answer
  (#496: 80 of one game's 98 calls, 85% of its prompt bytes, 72% of its wall
  clock, and up to 60 subprocesses and ~5h at the 300s default — all inside ONE
  watchdog observation, on a game that is not stalled).
  Know the watchdog's shape before designing an arm. `progress_fingerprint`
  (`mtg-player/src/watchdog.rs`) is board-only — no counters, no attachments, no
  mana colour, no combat assignments, and not `awaiting_action` itself, which is
  exactly why a cast-cancel cycle reads as identical — and it keeps only the
  *previous* value and resets `stalled` on any change, so any cycle of period
  >= 2 is invisible to it by construction. The prompt that would exploit that is
  the mulligan, whose every refusal reshuffles and redraws seven; it is bounded
  today by `mulligan_is_dominated` in one seat rather than by the runner. Two
  more things a future probe should not have to rediscover: the menace loop is
  reachable only through Terror of Kruin Pass, so the board needs a turn in
  which neither player casts a spell (give both decks four castable spells and
  56 cards they cannot pay for), and `--check-invariants`' only "the game is
  stuck" line can never execute — `engine.rs:1121/1276` short-circuit
  `offers_nothing()` before every callback, and the branch they take instead
  calls no callback, counts no action and logs nothing (#498). The third surface
  came free: the fuzzer's seat ignores `legal_blocks` and `min_blockers`
  outright, so 11 of 255 declarations were blocks the engine silently dropped
  and a menace minimum is never satisfied on purpose (#497)
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
  — **wrinkle added 2026-09-14, from #494**: the sharper question is not which
  fields one table names that the other drops, but which `Action` variants have
  no arm in `format_single_action` *at all* and land in
  `other => format!("{other}")`. `ActivateLoyaltyAbility` was one — the only
  `Display`-produced label of 367 shapes in a 2,609-request harvest — and the
  tell is that the engine's `Display` drops its `targets`, so two different
  actions become one row and a self-targeting ultimate is indistinguishable from
  one aimed at the opponent. Enumerate the variants `choose_action` can put in a
  row, find the ones with no arm, and for each ask whether the variant carries a
  field (`targets`, `sacrifice`, `x_value`) that the row then cannot express. A
  duplicated row in a harvest is the cheapest way to spot it
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
  — **played 2026-09-14: the tenth caller exists, its schema is sound, and the
  prompt wrapped around it is not.** `choose_pile_division` needs
  `4 Liliana of the Veil / 56 Swamp` vs `60 Island` and a stub policy that
  *wants* the ultimate — cast at 3 loyalty, `+1` three times, `-6` around turn
  19 — and it was then reached three times a game in a release build, along with
  `confirm_concede`, which no label-reading policy will ever reach because every
  seat is written to avoid `Concede`. That is all ten callers plus both draft
  schemas, 2,609 requests over eleven games and a draft, and the audit is
  **clean**: 0 illegal top-level keys, 0 empty enums, 0 `maxItems: 0`, 0
  `minItems > maxItems`, 0 `required` field missing from `properties`, 0 of 26
  `mark_indices` count-notes disagreeing with their `minItems`/`maxItems`, 0 of
  13 X-funding prompts disagreeing with their own stated X range, and 0 index
  enums naming a row the prompt body does not print. One direct
  `claude -p --json-schema` call with the real 24-key harvested pile schema came
  back with a complete answer, so the nested keys (spaces, `#`, and now `/` from
  the `0/0` suffix) are accepted *and* answerable. H13 was also right that the
  release binary checks nothing: `Cargo.toml` has no `[profile.release]`, so both
  `debug_assert!`s are compiled out and the harvest is the whole enforcement.
  The opponent's side of a pile division is not a structured prompt at all —
  `ChoosePile` is two enumerated `ChosenIndex` rows through `pick_action_index`.
  What the night found is one layer out. `format_single_action` has no
  `ActivateLoyaltyAbility` arm, so the row falls through to
  `other => format!("{other}")` and the engine's `Display`:
  `Activate loyalty ability 2 on obj#1`, the only `Display`-produced label of
  367 shapes harvested, with no name, no loyalty cost, no effect text, and the
  `targets` dropped — which makes "-6 at the opponent" and "-6 at yourself"
  byte-identical (33 of the harvest's 35 duplicated-row prompts; #494, the #61
  fix not crossing exactly as #118 did not in #460). The seat took the
  self-targeting copy, divided its own 24 Swamps and sacrificed all of them —
  and the prompt it got there said only "set true for pile 1, false for pile 2",
  naming neither whose permanents these were nor that the *target* player
  chooses a pile to sacrifice, which is the fact that inverts the answer, while
  every land read `0/0` (#495). The generalisation worth keeping: **a shape
  nothing reaches is worth building, but the audit that pays for itself is the
  one on the prompt, not the schema** — and the cheapest instrument for it is a
  record of the action-list labels, since a duplicated row and a `Display`-shaped
  label are both greppable in one pass. 16 of 17 `ResolutionChoiceKind` variants
  were raised (Nevermore's `ChooseCardName` took its own
  `4 Nevermore / 56 Plains` deck and is 253 numbered rows against a 253-value
  enum); `ChooseDamageEffect` is the one nothing reaches now, and is H16
- H14 [proposed 2026-09-14, from #496 and H11's measurements] the budget for
  ONE decision. Both bounds on a game are counted in decisions — the progress
  watchdog's `STALLED_DECISIONS` and `mtg-runner`'s `max_actions` — and a single
  decision can hold a loop inside it: `choose_blockers_structured`'s
  `max_retries = 20`, each retry a full `send_message_structured`, each of those
  up to `MAX_ATTEMPTS = 3` subprocesses bounded only by `CALL_TIMEOUT` (300s).
  That is 60 launches and about five hours on one prompt, and the watchdog
  observes it once, on a game that is not stalled. Method: walk every loop in
  `mtg-player/src/llm.rs` that can re-ask the *same* decision — the blocker
  validator, `call_once`'s retries, and anything a fix adds — and for each build
  a board that makes it run to exhaustion, then measure calls, prompt bytes and
  seconds per decision (a `sleep` in the `CLAUDE_CODE_BIN` stub turns the shape
  into a number). Then ask the question that produced it: which prompts have a
  client-side validator that can reject an answer the SCHEMA still permits?
  Every one of those is a retry loop waiting for a deterministic seat, and the
  fix is usually to narrow the answer rather than to ask again — the engine's own
  `declare_blockers_with_registry` drops the offending pairs and keeps the rest
- H15 [proposed 2026-09-14, from H11's mulligan arm] the refusals the watchdog
  can never see. `progress_fingerprint` catches a livelock only while the board
  stands still, so a prompt whose unusable answer CHANGES the state is invisible
  to it by construction — and it keeps only the previous fingerprint, resetting
  on any change, so a cycle of period >= 2 is invisible too. The mulligan is the
  archetype: CR 103.4 caps nothing, `legal/awaiting.rs` offers the mull
  unconditionally at every count, and each one shuffles and redraws seven, so a
  seat answering "mull" forever resets the fingerprint every decision and would
  run to the 50,000-action cap. It is bounded today by `mulligan_is_dominated`
  in `mtg-player/src/llm.rs` — a policy floor in ONE seat, and the comment
  beside it says so. Method: list every prompt a seat can refuse where the
  refusal mutates state, and for each check whether a floor exists in all four
  answering surfaces (`llm.rs`, `random.rs`, `cli.rs`, and
  `mtg-draft-runner`'s loop). Then check the remedy rather than assuming it: the
  draft runner forfeits a stalled game by returning `Concede`, and
  `legal.actions` is `vec![]` for all four structured prompts
  (`engine/legal/awaiting.rs:298-307`), so its forfeit and its action cap both
  depend on a flat action that is not there — `mtg-runner` returns
  `AbandonGame` unconditionally and does not
- H16 [proposed 2026-09-14, from H13's variant sweep] the seventeenth prompt.
  `ChooseDamageEffect` (CR 616.1) is the one `ResolutionChoiceKind` of seventeen
  that 6,150 harvested structured requests over twenty-two games and a draft
  never raised: it needs two replacement or prevention effects applying to one
  damage event with the order changing the outcome, and no deck pair in `decks/`
  puts two on the board at once. Build it — a one-off deck with Inquisitor's
  Flail (`DamageEffect::Double`) on an attacker whose damage is also being
  prevented (Ghostly Possession on either end for `PreventAll`, or Unbreathing
  Horde as the blocker for `PreventAndRemoveCounter`; the variants are at
  `mtg-engine/src/damage.rs:74`) — and read the two `ChosenIndex` rows the LLM
  seat is handed against the CLI's screen and the random seat's uniform pick.
  Verify three things: that each row says what that effect WOULD do rather than
  naming the permanent, that the re-raise after the first answer is not the same
  row again (#323 says the remaining effects are asked about only while the
  order still matters), and that the effect the seat picked is the one applied.
  A prompt no game has ever raised is a prompt no night has ever read
- H17 [proposed 2026-09-14, from H7 and the comment at
  `mtg-player/src/llm.rs:3178`] the same schema, two providers. The X-funding
  builder deliberately encodes integers as string enums, with a comment stating
  why: *"Anthropic rejects `minimum`/`maximum` on integer fields, Gemini rejects
  `enum` on integer fields, and only `enum` on string fields is both accepted
  and enforced by both."* The declare-attackers schema (`llm.rs:4093-4113`) then
  uses `{"type": "integer", "minimum": 0}` for `attacker_indices.items` and for
  both fields of `planeswalker_attacks`, and the mulligan-bottom and
  `mark_indices` shapes use bare integer `enum`s. One of those two things is
  wrong. A `claude -p --json-schema` probe on 2026-09-14 showed the CLI path
  accepts *and* answers the attackers schema with `minimum` in it, so if the
  comment is right about the raw API then the metered `claude` seat cannot
  declare attackers at all while the `cc` seat can — #398 exactly, on the one
  prompt a game cannot progress without. Do it without spending a metered seat:
  harvest every top-level shape with a `CLAUDE_CODE_BIN` stub, then check each
  against what each backend's own code claims its provider enforces
  (`mtg-player/src/llm/` and the comment above, plus
  `sanitize_schema_for_anthropic` and whatever the draft path does), and say for
  each shape which backends can carry it. Where the answer is "the comment is
  stale", the string-enum workaround can go with it; where it is "the schema is
  wrong", that is a defect in a seat this crew is forbidden to run, which is
  exactly why nobody has noticed
