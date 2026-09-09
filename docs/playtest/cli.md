# Playtesting the CLI

Subject: the machine — `mtg-runner` and `mtg-player`'s interactive
surface as *programs*. The TUI, the flags, the files they write, the
signals they receive, the terminals they run in. Not the rules; a game
that follows the CR perfectly can still lose your save, leak the
opponent's hand into a log, or wedge on a resize.

## Before you start

Read `mtg-runner/tests/cli_pty.rs` first — those contracts are already
pinned in CI, and re-finding them is a wasted night. What you can do that
CI can't is real terminals, real signals, real filesystems, hostile
timing, and games long enough to drift.

The ideas below are a starting point, not a syllabus. They are what
previous nights happened to think of, and the bugs that mattered most
were usually not on the list when the night began. The real method is the
one underneath them: read the code that implements this, read the rule or
the contract it is supposed to satisfy, and find where the two disagree.
When you find a way to look that the list doesn't have, take it — and
then add it, per "Adding an idea" in `docs/playtest/README.md`.

## Where to look

- `mtg-player/src/cli.rs` is the interactive surface: prompts, panes,
  input parsing, rendering. `mtg-player/src/game_log.rs` is `--log`.
- `mtg-runner/src/main.rs` is the flags, the seat types, save/resume, and
  the argument validation that decides what is a clean error.
- The `play-cli` skill documents the house tmux patterns:
  `tmux new-session -d -s <name> -x <cols> -y <rows> '<cmd>; sleep 300'`,
  `send-keys`, `capture-pane -p`, `resize-window`.
- The contract here is not the CR, so you have to decide what correct
  means. Useful questions: would a user be surprised? Is the failure
  clean and explained, or a panic? Does the program do something
  irreversible the user didn't ask for? Is anything visible that the
  player at the keyboard is not entitled to see?

## Ideas

**The Vandal** plays both seats to break it. Wins don't matter; panics,
hangs, stuck prompts, corrupted state and nonsense output do.

- V1 input garbage at every prompt: junk text, huge numbers, empty
  enter, unicode, control characters (game must reprompt, never crash)
- V2 the wrong number: at every numbered menu, try -1, 0 off-by-one,
  and N+1 before choosing legally
- V3 save/reload abuse: `--save` then `--resume` mid-combat, mid-choice,
  mid-mulligan; resume the same save twice; `rr` hot-reload at odd times
- V4 degenerate decks: all-curses, zero-creature, 4x same legend,
  token-flood (Army of the Damned + doublers), one-of-everything piles
- V5 stall: durdle to turn 100+, empty attacks, verify draw-out and
  deck-out endings actually end the game
- V6 concede at the weirdest legal moment: mid-choice, during combat,
  with triggers on the stack
- V7 does everything printed FIT? "Look for broken rendering" scheduled
  nothing, and this is the single richest vein the subject has: #318 (menu
  rows head-clipped), #350 (header and life lines printed unclipped, erasing
  the pane border at 100 columns and wrapping into the STACK pane at 70),
  #351 (combat hints over the border and onto the prompt row), #352 (at 70x20
  the combat prompts show NO creature list and still accept a declaration),
  #355 (the input row erases a line of the CARDS pane every frame), #364 (`i`
  has no pager at all — half the board unreachable at 32 permanents), #365
  (pagers count entries while printing wrapped rows, so a page scrolls its
  own heading away), #366 (`/` silently dead below 100 columns). Sweep it as
  a contract rather than a rummage. Three properties, each checkable: every
  line a pane prints is inside that pane and inside the terminal; a list
  longer than the body pages, and the pager can reach its LAST entry; a page
  is sized in rendered lines, not in entries. Build the stressors —
  `20 Armored Skaab / 20 Makeshift Mauler` for long names, a 40-card
  graveyard, 30+ permanents a side, a nine-trigger upkeep — and walk every
  screen at 70x20, 80x24, exactly 100 wide, and 200x50, `capture-pane`-ing
  each. `draw_set_screen` is the one to copy from: it wraps rows into lines
  FIRST and pages the lines, which is why it has none of these
- V8 search/menu abuse: the CLI's `/` search, `d`, `l`, `g`, `e` panes
  spammed at every prompt
- V9 rapid concede/new-game churn: concede and immediately relaunch a
  fresh game back-to-back many times in the same session; verify no
  leaked state (stale board/log/hand) bleeds into the next game
- V10 priority-mash marathon: hold pass-priority/`f` auto-pass through an
  entire game from turn 1 to conclusion; verify no mandatory decision
  (declare attackers/blockers, discard to hand size, trigger ordering) is
  silently skipped and nothing double-resolves
- V11 terminal resize storm: resize the pane aggressively mid-game and
  mid-prompt (tiny, huge, back), including with a target-selection or
  declare-blockers prompt open and a deep stack; look for panics,
  unrecoverable frames, unreachable prompts, misrouted input
- V12 control-character and escape-sequence injection: send Ctrl
  chords, Escape, arrows, function keys, Tab, Backspace-on-empty and
  literal ANSI sequences at every prompt type; nothing unbound may be
  inserted as text or dispatched as a menu shortcut
- V13 paste-flood: paste multi-KB single lines and 50-line blocks at
  every prompt; watch whether queued lines are consumed as independent
  menu submissions and silently take real, irreversible game actions
- V14 save/resume corruption abuse (distinct from V3's honest
  save/reload): resume from truncated, byte-flipped, empty,
  wrong-schema and structurally-invalid saves, and with mismatched
  decks/seed; failures must be clean errors, never panics, and never a
  silently-wrong game
- V15 mulligan-phase abuse: mulligan to the floor on both seats, find
  the real cap and check the counter is honest, send garbage and
  out-of-range input at every mulligan and bottoming prompt, and verify
  a floor-mulligan game is still playable to a conclusion
- V16 deck-file abuse: empty, comments-only, zero/negative/overflowing
  counts, unknown and unicode card names, missing counts, no separator,
  duplicate lines, binary bytes, CRLF, absurdly long names; every failure
  must be a clean error with a non-zero exit, never a panic
- V17 CLI flag abuse: bad/negative/overflowing --seed, missing flag
  values, unknown flags, unknown player-type values, --resume on a
  missing file or a directory, and --log/--save pointed at unwritable
  paths, directories and /dev/full
- V18 EOF, signals and terminal detach: Ctrl-D at every prompt type,
  SIGINT/SIGTSTP+CONT/SIGHUP mid-prompt, tmux detach and reattach with a
  prompt open; nothing may busy-loop, silently choose an action, leave
  the terminal in raw mode, or survive into a corrupted game
- V19 type-ahead race (distinct from V13's paste-flood): many separate
  legitimate keystrokes sent faster than the render loop, including
  bursts that straddle a prompt-type or seat change; hunt for input
  consumed against a prompt the player was never shown
- V20 concurrent save contention (distinct from V3's honest reload and
  V14's corrupted saves): two live games writing one --save path,
  resuming a save while its writer is still writing, resuming one save
  into two processes, and save paths that are directories, read-only or
  /dev/full
- V21 nested-prompt abuse (distinct from V8's top-level pane spam): send
  the pane shortcuts, bare Enter, out-of-range indices and junk into the
  SUB-prompts — target chooser, X cost, chooser filter box, blocker
  assignment, trigger ordering, "may" yes/no, mulligan bottoming, concede
  confirmation — and verify a pane opened inside a nested prompt returns
  to that same prompt with the same state
- V22 marathon: drive one game past turn 150 with both seats durdling;
  watch the turn counter and step header, the log panel and `l` view at
  extreme length, 40+ card graveyard panes, save-file growth, RSS and CPU
  over the run, pager behaviour on a 1000+ entry log, per-input latency
  drift, invariant failures, and the exact turn and cause of the deck-out
  ending
- V23 structured-prompt syntax abuse: the declare-attackers and
  declare-blockers parsers take free-form text — feed duplicate indices,
  "all none", "0:0:0", ":0", "0:", one valid plus one invalid entry,
  5000-character index lists, and mixed separators. Anything not fully
  valid must be refused with an error and a reprompt, never trimmed to a
  partial declaration or read as "none"
- V24 hostile environment and non-TTY execution: stdin at EOF, piped
  stdin, stdout to a file or pipe, TERM=dumb and TERM unset, COLUMNS=1,
  a backgrounded process taking SIGTTIN, SIGSTOP/SIGCONT mid-prompt,
  SIGWINCH storms, and a 1x1 pane at startup; each must be a clean
  explained exit or a game that keeps working
- V25 search and filter-box abuse: regex metacharacters, format-string
  payloads, 10000-character strings, empty and all-matching searches,
  unicode (combining marks, RTL override, ZWJ, CJK, emoji) and ANSI
  sequences at the `/` search and inside a chooser's filter box; never
  crash, hang, corrupt the frame, become unexitable, or mis-scope to a
  zone the searching seat can't see

- V26 the action list as a contract: build a priority with 30-60+ legal
  actions (many land types, castable spells, activated abilities, equip
  costs) and check that the displayed list and the engine's `legal_actions`
  still agree. Does index N do what line N says, at the last index, past the
  end, and across a page boundary? Is the user TOLD when the list is clipped,
  and can they still reach what is hidden? Do collapsed duplicate lines act on
  the right object when the sources are not interchangeable? Is Concede always
  at the index the screen claims? Read `CliPlayer::choose_action` →
  `render_paged` → `clip_middle` first, and vary the pane HEIGHT — a short
  pane is what breaks it. #209 is the LLM-seat face of this question; the
  `cli` seat does not share that defect, but four others live here
  (#257, #258, #260, #261)
- V27 the post-game state machine: reach a game over by all three routes —
  concede, lethal damage, and deck-out — and ask what the program does
  AFTERWARDS. Exit code per route; input sent after the game is decided;
  whether the game-over screen agrees with the tail of `--log`; what `--save`
  holds at the end and what `--resume` on it does; SIGINT and SIGKILL at the
  game-over instant; and whether the terminal is left out of raw mode and off
  the alternate screen. Every previous Vandal night attacked a RUNNING game
- V28 does the screen tell the truth about the object? Distinct from V7's
  rendering-at-scale: this is rendering ACCURACY. Build permanents whose real
  characteristics differ from their printed ones — a transformed DFC, a
  creature with counters under an anthem under an aura, a graveyard-CDA `*/*`,
  a granted keyword, a token, an artifact creature, a creature attacking or
  blocking — and check the battlefield line, `i` inspect, the CARDS reference,
  `/` search, `d` and `g` all agree with the `--save`, which is the engine's
  own view and the tiebreaker. A pane that lies is worse than a crash because
  the player acts on it
- V29 stack depth as the stressor: L1 checked deep stacks as a RULES question;
  this asks what the MACHINE does at depth. Get 90+ objects on the stack (a
  sweeper into a board of death-triggers is the fastest route), then read the
  STACK pane against the save, verify strict LIFO all the way down (CR 608.1),
  resize, save/kill/resume, concede, and feed garbage and paste floods to both
  the priority prompt and the trigger-ordering prompt. Note the ordering prompt
  is a repeated single choice, not a permutation, so duplicate and partial
  permutations are not expressible
- V30 every save the game ever wrote must be loadable: `--save` rewrites the
  file at every decision, and V3/V14/V20/M1 only ever looked at moments
  somebody hand-picked. Snapshot the live save on a tight loop for a whole
  game, then `--resume` every distinct snapshot. Also: is any snapshot ever
  torn or zero-length; does a SIGKILL mid-write lose the file; does the save
  lag the screen; does a resume from an arbitrary snapshot land where it
  claims; and does the file grow without bound. The atomic-write fix from
  #75/#76 holds (2,173 of 2,173 snapshots resumed on 2026-09-05) — the defects
  are around the file, not in it (#239, #242)
- V31 [tried 2026-09-06 → #294, #295, #296] auto-pass as a contract (distinct
  from V10's priority-mash): `f` promises "passes until your next Main Phase 1"
  and four break conditions. Read `CliPlayer::should_break_pass` and
  `try_engage_auto_pass` first, enumerate the clauses, then build a state for
  each and check the code, the header label and `play-cli.md` all agree. Ask what
  is silently declined (a cast, an equip, six ability modes), where the mode can
  be seen (only on mandatory-choice screens), and how it is turned off (it
  isn't). Mandatory decisions are safe — the auto-pass path is gated on
  `has_pass`, and every mandatory choice in this program lacks a Pass option — so
  hunt the contract, not the rules. #269 is what makes the extra stops visible at
  all, and fixing it will HIDE #295's stops without removing them
- V32 [tried 2026-09-06 → #281, #282, #283, #284] the input line as an editor:
  is the string rendered after the `>` the string Enter submits? Read `read_line`,
  `read_line_with_search` and their `echoed_chars`/`echoed_cols` bookkeeping
  first, then break it at every prompt — Backspace on full and empty buffers,
  Ctrl-U, a line past the echo cap, and multi-byte text deleted one keypress at a
  time (CJK, a combining mark, a ZWJ emoji: three `char`s, one cluster, three
  different erase widths). The end state to hunt is a line that renders EMPTY
  over a non-empty buffer, because Enter then means "attack with everything" at a
  prompt that says `enter=none`. Also check what the echo and the `Invalid input
  '…'` notice do to bytes they were handed — neither strips control characters
  nor clips the notice — and whether the buffer survives an error reprompt, a
  pane, a resize, a seat change and `--save`/`--resume` (it does; the echo is
  what does not). The `/` search box and the chooser filter box redraw from the
  string each keystroke and are immune, which is the shape of the fix. Unreached:
  the trigger-ordering prompt
- V33 [tried 2026-09-06 → #288, #290, comments on #249/#254/#261/#262] the escape
  hatch: enumerate every prompt from `cli.rs` — action menu, the three
  `prompt_target*` choosers, `prompt_x_funding`, `prompt_exile_from_graveyard`,
  `prompt_pile_division`, `library_search_ui`, `choose_attackers`,
  `choose_blockers`, `confirm_yn` — and for each ask three questions: is there a
  way to back out and does it work, what does a bare Enter do and is that written
  on screen, and what has irreversibly happened once you commit. The 2026-09-06
  sweep found the idle key means "cancel" at one chooser and "cast the spell" at
  two others, that `prompt_target_optional` has no Cancel row at all, and that an
  ability's X cost is paid before the prompt that asks for X. Re-run it after any
  prompt is added or a cancel is wired up; the unswept corners are
  `ChooseCardType`, `ChoosePile`, `ChooseCardName` and an X ability that also
  carries a sacrifice cost
- V34 [tried 2026-09-06 → #293] what the save forgets (distinct from V3's honest
  reload, V14's corrupted saves, V20's contention, V30's every-snapshot-loadable
  and M1's faithful screen — all of which ask whether the save LOADS, not whether
  it round-trips every FIELD): read the struct that gets serialized before you
  play and hunt a `GameState` field that is not in it. Then reach a state thick
  with ephemeral, turn-scoped state — damage marked, an "until end of turn" pump
  or protection or control change in flight, the land drop spent, floating mana,
  declared attackers and blockers, a once-per-turn ability already used (a
  planeswalker's loyalty, CR 606.3), the werewolf "spells cast last turn" count,
  morbid, summoning sickness, the mulligan count — save, kill, `--resume`, and
  compare. The behavioural check outranks the screen: does the pumped creature
  still hit for the bonus, can you play a SECOND land, does the werewolf still
  transform on schedule, does one more point of damage still kill the creature
  that was already wounded. Cheapest rigorous form: let the resumed process write
  its own save at the same decision and deep-diff the two. On 2026-09-06 all 15
  probes came back identical — `GameState` round-trips; what `--resume` restarts
  rather than restores lives OUTSIDE it (#248 the seats, #293 the action
  counter), so look there next
- V35 [tried 2026-09-06 → #287, #289, #291, comment on #260] the refusal: when
  the machine says no, does it say why? Feed every prompt the PLAUSIBLE illegal
  input a real player would try, not V1/V2/V23's garbage — a blocker assigned to
  a flyer, `N>pwM` with M out of range, a blocker index that is a live ATTACKER
  index, X above max, "none" with a must-attack creature — and check each refusal
  names the object AND the reason, survives to the reprompt, refuses the
  declaration whole, and leaves `--log` and `--save` byte-identical. Nothing
  illegal is ever OFFERED (targets, loyalty abilities, sick attackers,
  unaffordable casts and defenders are all pre-filtered), so the bugs live
  entirely in the wording: read `choose_attackers`'s `bad` list,
  `choose_blockers`'s parse-and-range match arm, and the `show_error` closure's
  sleep-then-clear before you play. Unreached: a menace/`min_blockers` refusal,
  and a protection-based TARGETING refusal

- V36 [tried 2026-09-07 → #325, #326, #327] the trigger-ordering prompt as a
  reader: `ChooseTriggerOrder` (CR 603.3b) is the last unswept input in the
  CLI and the only one that takes an ordering. Read `triggers.rs`'s
  `distinguishable` gate and `choices.rs`'s `ChosenIndex` arm first — the
  prompt is a REPEATED single index, re-raised until only interchangeable
  triggers remain, so duplicate and partial permutations are inexpressible
  and V23's #108 shape cannot recur. Cheapest big group: N Unruly Mobs dying
  to one Rolling Temblor is N(N-1) distinguishable triggers (four Mobs = 12);
  both players trading 1/1s in combat gets AP and NAP groups in one event.
  The reader itself is clean — 24 hostile inputs all refused with a named
  notice, `--save`/`--log` byte-identical, panes return to the same prompt —
  so hunt the SHELL: `choose_action`'s `naming_cards` gate (`card_count > 8
  && all ChosenIndex`, added for Nevermore in #255) also swallows this prompt
  and `ChooseDamageAssignmentOrder`, so past 8 options the ordering becomes
  the card browser with no board, no stack and no shortcuts (#325), and the
  `--log` writes N identical lines for N distinct choices because #116's
  `[source #id]` tail never reached it (#326). Both fixed: both ordering
  prompts now take over the screen — every trigger with its source, P/T,
  ability and cause, the stack, pane shortcuts that come back to the same
  prompt — and take the whole order as one list of numbers (bare Enter keeps
  the order shown); the log names each trigger's source by id. Re-probe the
  new reader with V1-style garbage: repeats, gaps, out-of-range numbers, a
  number followed by a letter, and pane keys typed mid-list. Verify the answer with two
  `--resume`s of one saved trigger prompt answered in opposite orders — the
  save round-trips the prompt exactly. Unreached: an ordering whose FINAL
  board state differs (no order-dependent trigger pair is cheap in ISD; try
  Curse of the Bloody Tome on yourself plus Delver of Secrets at one upkeep),
  a 9+ `ChooseDamageAssignmentOrder`, and Ctrl-C/Ctrl-D at the prompt
- V37 [tried 2026-09-07 → #318, #320, #321, #322] the cardinality prompts as a
  family: every question that takes a SET under a constraint — exactly N, up to
  N, at least one. Read the readers first, because there is almost no reader:
  `legal/awaiting.rs::BottomAfterMulligan` and `cards_flow.rs::legal_discard_actions`
  both call `combinations()` and hand the CLI one menu row per COMBINATION, so
  bottoming and cleanup discard are the single-index menu (V26's reader) with
  N-card labels; Brain Weevil's "discards two cards" is `library_search_ui` run
  twice; "up to N targets" is a chain of `prompt_target_up_to` single choosers;
  and the only genuine multi-select is `prompt_exile_from_graveyard` (Skaab
  Ruinator exactly-3, Skaab Goliath 2, Makeshift Mauler/Stitched Drake 1,
  Harvest Pyre min=0/max=graveyard). Reach them with one-off decks: 15
  long-named cards for bottoming, `20 Island / 20 Armored Skaab / 20 Makeshift
  Mauler` for the exile cost, Dream Twist + Memory's Journey for up-to-3. Send
  each empty, short, long, duplicated (`0 0 1`), out-of-range, negative,
  `99999999999999999999`, comma-separated, double-spaced, tabbed, 4 KB, and the
  panes. The reader itself is sound and the chosen set always reconciled against
  `--save`; the bugs are around it — the enumerated rows are built with
  `MenuLabel::plain`, so they carry no ids and `clip_menu_page`'s #136/#258
  disambiguation cannot fire while `fit_menu_label` head-clips exactly the card
  that distinguishes them (#318); the exile hint is 61 columns in a 58-column
  panel (#320); `7 - mull_count` underflows past seven mulligans (#321); and an
  unbound key dropped between two digits fuses them into a third accepted index
  (#322). Pane WIDTH is the variable for #318/#320 and 100 columns is the worst
  case, not 80. Unreached: a cleanup discard of 2+ (needs a hand of nine, which
  the ISD pool would not give), `prompt_pile_division` (Liliana's ultimate),
  Divine Reckoning, Forbidden Alchemy, Mulch, Make a Wish, Creeping
  Renaissance, Ghoulcaller's Chant, Moan of the Unhallowed, Sever the
  Bloodline, Grimoire of the Dead, and Skaab Goliath's exactly-2.
  **Superseded 2026-09-09**: `combinations()` is gone from the engine and
  every question in this family is now one marking screen (`pick_set` in
  `cli.rs`) — a numbered list you toggle with `1 3 5`, plus `a`/`n`/Enter.
  The enumerated-row bugs above cannot recur, but the screen is new and
  unplayed: try it at a 15-card bottoming, at Skaab Ruinator's exactly-3,
  at an "up to two target creatures" (Travel Preparations, Feeling of
  Dread), and at `prompt_pile_division`, with the same hostile inputs
- V38 [tried 2026-09-07 → #313, #314, #315, #316, #317] the resume boundary as
  a state machine (distinct from V3's honest reload, V14's corrupted saves,
  V20's contention, V30's every-snapshot-loadable and V34's every-field): the
  save is valid, the surrounding REQUEST is not. `--resume` takes a file plus a
  fresh argv and the two can disagree. Read `main`'s resume block first — it
  narrates `--deck1/2`, `--seed`, `--p1/2` and stays silent on the rest. Then:
  resume one snapshot into two concurrent processes and play them divergently
  (independent and correct); resume it twice in sequence (diverges, so state is
  restored not replayed); chain six resumes of 5 decisions with
  `--resume X --save X --log X` and deep-diff the final save against one
  uninterrupted 30-decision run (0 fields differ — GameState round-trips);
  SIGKILL mid-play 20 times and resume whatever landed (20/20, no torn or
  zero-length file, no stray `.tmp`); resume from a directory, /dev/null, a
  zero-byte file, a half-truncated save, a dangling symlink and a missing path
  (all exit 1, honest message, no panic). What breaks is everything the save is
  NOT: the `--log` high-water mark restarts at 0 so the whole history is
  re-emitted under fresh timestamps (#313), a decided game resumes instead of
  refusing (#316), `SaveData.seats` picks the player backend and the model with
  no flag and `--quiet` deletes the only line that says so (#314),
  `player_names` reaches stdout and the log unescaped (#315), and no `--save`
  means the file you resumed from silently freezes (#317). Ask of every field
  in `SaveData`: who controls it, and what does the runner PRINT it into.
  Unreached: resume against a mid-write reader (V20's ground), the LLM
  `resume_from_log` recap path (no metered seats), and whether any
  save-sourced string reaches a PROMPT rather than the banner
- V39 [tried 2026-09-08 → #350, #351, #352, #353, #354, comment on #318] the terminal
  geometry matrix (distinct from V11's mid-game resize storm — this is STATIC size ×
  prompt type): reach a screen, then render it COLD at 300x10, 200x50, 120x45, 100x30,
  80x24, 70x20, 60x15, 40x12, 40x80 and 20x5, and read every one with `capture-pane -p`
  measuring each row's display width and the columns holding a `│`. `--save`/`--resume`
  is the cheap way to put one position on many terminals; cold-starting at the size is
  what separates a static defect from a resize one, and both exist. Read `render_paged`'s
  `has_right = w >= 100` first: the CARDS gutter switches on at exactly 100 columns and
  the middle panel drops from 63 (at 80) to 58, so 100 is narrower for content than 99
  and is the worst width in the program — #318, #350 and #351 all live there. Everything
  in `render_paged` goes through `clip_cols` except three lines (`status`, `opp_stats`,
  `your_stats`), and the two combat prompts clip nothing at all, so hunt unbounded
  `Print`. Below 70 columns the combat prompts stop drawing their creature lists entirely
  while still committing a declaration (#352), and both freeze `col`/`term_w` outside
  their redraw closure (#353). 200x50 and 120x45 were clean on every screen; the program
  survived 20x5 and 1x1 and came back. Unreached: the trigger-ordering prompt,
  `prompt_x_funding`, `library_search_ui` and the `/` search box at any size; a CJK/emoji
  card name against the clip arithmetic; and whether a minimum-size refusal would beat
  the 20x5 frame
- V40 [tried 2026-09-08 → #360, #361] the hotseat seat-switch boundary: in
  `--p1 cli --p2 cli` two players share one terminal, so read `GameView::for_player` and
  every `view.you` in `cli.rs` and then hunt a viewpoint that is stale, wrong or absent —
  both mulligan decisions and the BOTTOM-N prompt, priority on the opponent's turn,
  declare blockers, a target chosen for an opponent's trigger, and `d`/`g`/`e`/`l`/`i`/
  `s`/`/` opened as each seat in turn. Capture at `sleep 0.1` as well as 0.8 to catch a
  frame that has not re-scoped yet. The 2026-09-08 sweep found the RENDERING clean — the
  view is per-seat, `render_paged` opens with `Clear(All)`, `LogLevel::Private` keeps the
  look-at out of `--log`, the mulligan bottoming is logged unnamed, and the deck browser,
  search box and CARDS gutter read own-hand plus public zones only — so the boundary
  worth attacking is INPUT, not pixels: `LAST_DECISION_IDENTITY` is written only by
  `choose_action`, so `choose_attackers`/`choose_blockers` are invisible to #71's drain
  and one seat's stray keystroke lands on the other seat's menu (#360), and `confirm_yn`
  has no drain at all (#361), so the two compose into a burst that concedes the opponent's
  game with no prompt ever drawn. Unreached: a cross-seat `library_search_ui` (the one
  reader with no drain), and whether `--resume` re-seats a burst mid-flight
- V41 [tried 2026-09-08 → #364, #365, #366, #367, comment on #362] the eight side viewers
  as a SET: `d` `g` `e` `l` `i` `s` `/` and `m`/`b`/`p` are advertised on the hint line at
  every prompt and had never been audited as a contract — a viewer is read-only, available
  where advertised, returns you to exactly the decision you left, and tells the truth.
  Read `menu_hints`, `parse_target_input`'s `"lgedis/"` set, `show_paged_lines`/
  `page_window` and `show_battlefield_inspector` first, then sweep every viewer against
  every prompt: `cp` the `--save`, open, close, `diff` (a paused prompt is quiet — the
  file is only written at a decision), and `capture-pane`-diff the return. Read-only and
  return HELD everywhere across 9 prompt kinds and 60+ opens, including across a
  `--resume`; escape is uniform (Enter closes, an unrecognised key inside a viewer closes
  it and never leaks through, nothing nests or sticks); and both leak questions came back
  clean (`d` aggregates by name and hides library order, `/` only searches zones the seat
  can see). The bugs are all presentation: `i` has no pager at all (#364), both pagers
  size pages in entries while printing wrapped rows (#365), `i` is accepted-but-
  unadvertised at every menu while `/` is silently dead below 100 columns (#366), `g`
  orders its blocks by seat where `e` anchors them to "you" (#367), and the stack view
  drops the ids the chooser and ordering prompt print (#362). Unreached: declare blockers,
  `prompt_exile_from_graveyard`, and `library_search_ui` — which binds no viewer key at
  all, so a Forbidden Alchemy choice is made with no board, no graveyard and no log on
  screen

- V42 [proposed 2026-09-09, from #318, #398 and
  `mtg-engine/tests/prompt_shapes.rs`] how big can a question get? A prompt
  that offers one row per way of answering it grows as `C(n,k)`, `|a| x |b|`
  or `n + C(n,2)`, and every such prompt in the pool has now been converted
  to a marking screen or a slot-at-a-time ask — mulligan bottoming, cleanup
  discard, exile costs, pile division, "up to N" targets, ordered pairs,
  Ghoulcaller's Chant's modes, Curse of Oblivion's two cards. This asks
  whether that HELD, and whether anything else grows. Method: build the
  widest legal board for each prompt kind (a 12-Zombie graveyard for the
  Chant, 8 creatures a side for Prey Upon, 16 lands and 16 creatures for
  Into the Maw of Hell, a 40-card graveyard for Harvest Pyre), then count
  the `CastSpell` rows the menu carries and the rows the screen lists.
  Rows should track the number of OBJECTS, not their combinations. Read
  `prompt_shapes.rs` first — it sweeps requirement shapes and will already
  have failed on a new card, so what it cannot see is the interesting part:
  activated abilities, resolution prompts that ask N times in a row, and
  anything the CLI itself expands. Ask the LLM side of the same board in
  the same sitting: `format_action_prompt` joins every action into one
  comma-separated line with no cap, so a blowup that a person can page past
  is a prompt a model cannot read at all

**The Operator** neither plays to win nor tries to break anything: runs
the binary the way an operator would and checks it kept its promises.
The Vandal asks whether the machine survives abuse; the Operator asks
whether it told the truth.

- M1 save/resume fidelity: play N decisions, kill the process, `--resume`,
  and verify the resumed game matches what was on screen — board, life,
  hand counts, and the prompt it lands on — then that it plays out to a
  conclusion clean under `--check-invariants`. V3 and V14 abuse saves;
  this one checks an honest save is faithful
- M2 log and screen reconciliation: with two hotseat seats, verify every
  visible state change has a `--log` line and that the totals add up
  (life, cards drawn, mana spent, damage dealt), then comb the log and
  the save file for information one seat must not see. L25 covers the
  leak at the prompt; this covers it in the artifacts left behind
- M3 determinism from the outside: same seed and same scripted keystrokes
  twice must give a byte-identical `--log` (timestamps aside) and
  identical saves; `--on-the-play` honoured; different seeds actually
  differ
- M4 flag and file matrix: flags in combination rather than one at a time
  — `--resume` plus everything else — and against a filesystem that
  fights back: read-only directories, a small tmpfs filled to capacity
  mid-game, paths that are directories or symlinks. V17 abuses flag
  values; this walks the combinations and the capacity edge
- M5 audit `--help` against the
  code, sentence by sentence. Three of tonight's bugs were one line of usage
  text next to one line of implementation: `--log` says "Append" and
  `game_log::init` passes `.truncate(true)`; the `--resume` note names three
  flags and gets two wrong in opposite directions, while the one flag that is
  genuinely ignored (`--on-the-play`) is not mentioned at all. Take every
  claim `--help` makes, find the code that would have to be true for it, and
  run the command that distinguishes them. A flag that is accepted and then
  silently ignored, or documented as ignored and then used, is a broken
  promise and worth filing
