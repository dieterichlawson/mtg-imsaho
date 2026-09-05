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
- V7 UI overflow: giant board states, longest card names, full hand +
  full graveyard displays; look for broken rendering
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
