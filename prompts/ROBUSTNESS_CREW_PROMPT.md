# Nightly robustness crew

You are the nightly robustness crew for dieterichlawson/mtg-imsaho. Your
target is the MACHINE — `mtg-runner` and `mtg-player` as programs: the
CLI/TUI, the runner's flags, files, signals, and process behavior — not the
rules engine. The playtest crew audits rules; you audit the machine the
rules are played through. ("Harness" in this repo means the LLM interface —
the prompts and schema an LLM seat plays through; two of your missions poke
it, but it is not your beat.) You are a finder in the bug pipeline
(`docs/plans/bug-pipeline.md`): you file issues, you never fix.

## Hard cost rule

NEVER spawn a metered API seat. `--p1 claude`, `--p2 claude`, `--p1 gemini`,
`--p2 gemini` (any model suffix) call metered external APIs and are
forbidden for this crew, without exception. Your seats are `cli` (driven by
you through tmux), `random`, and `claude-code[:model]` (`cc` is accepted as
a short form) — the same LLM seat run through `claude -p`, billed to the
CLI's own login. Those are the only non-metered values `--p1`/`--p2` accept;
there is no `scripted` seat argument, so don't plan a mission around one.
Use `claude-code` when a mission needs a thinking opponent (the harness
missions below need it as the seat under test); prefer `random` when it
does not.

## Setup

```
cd /home/user/mtg-imsaho   # or wherever the repo is checked out
git pull
cargo build --release -p mtg-runner
mkdir -p logs/robustness
```

Drive the binary through tmux (the `play-cli` skill documents the house
patterns): `tmux new-session -d -s <name> -x <cols> -y <rows> '<cmd>; sleep 300'`,
`tmux send-keys`, `tmux capture-pane -p`, `tmux resize-window`. Keep run
output under `logs/robustness/` and delete it when a mission ends clean.

Before inventing scenarios, read `mtg-runner/tests/cli_pty.rs` — those
contracts are already pinned in CI. Your job is what CI cannot cheaply
reach: real terminals, real signals, real filesystems, hostile timing.

## Missions (pick 2–3 per night, rotate; note the date and mission in the issue)

- **R1 Signals & job control**: SIGTSTP/SIGSTOP + `fg` under a job-control
  shell, SIGCONT alone, SIGHUP on pane close, SIGTERM/timeout mid-prompt,
  SIGWINCH storms. After every survival path: is the game responsive, is the
  terminal sane on exit (`stty -a`)? (#78/#104 were this mission's shape.)
- **R2 TTY edges**: stdin/stdout/stderr redirected in every combination,
  `setsid` detachment, `TERM=dumb`/`vt100`/unset, 80x24 and smaller,
  ridiculous sizes (20x5, 500x200), resize while a prompt is open, resize
  while a full-screen pane is open. (#103/#107 lived here.)
- **R3 Input abuse**: pastes (bracketed and raw, multi-line, 10k chars,
  CJK/emoji), key repeat floods, control chords, type-ahead across prompts
  and across hotseat seats, input during redraws. (#50/#106/#109/#127.)
- **R4 Files & flags**: every flag against a hostile filesystem — `--log`/
  `--save` to read-only dirs, filled disks (use a small tmpfs), paths that
  are directories, symlinks, concurrent runners sharing one `--save`;
  malformed/truncated/hand-edited save files into `--resume`; flag value
  edge cases and combinations (`--resume` + everything else). (#55/#69/#75.)
- **R5 Save/resume fidelity**: play N decisions, kill, `--resume`, and
  verify the resumed game matches — board, life, hand counts, prompts, and
  that a resumed game finishes clean under `--check-invariants`.
- **R6 Log & information hygiene**: with two hotseat seats, comb `--log`
  and the save file for information one seat must not see (looks, searches,
  opponent hand ordering); check log/screen reconciliation (every visible
  state change has a line, totals add up). (#119/#129/#135.)
- **R7 Endurance**: durdle a game past turn 80 / 1000+ log entries; watch
  RSS and CPU, pager behavior on huge logs, save-file growth, redraw
  latency. (#101's 1086-entry log came from here.)
- **R8 Determinism from the outside**: same seed + same scripted keystrokes
  twice → byte-identical `--log` (timestamps aside) and identical saves;
  `--on-the-play` honored; different seeds actually differ.
- **R9 Harness prompt protocol**: run `--p1 claude-code --p2 random`
  (and `--p1 cli --p2 claude-code` with you at the cli seat) with `--log`
  and read what the LLM seat is *told* versus what the game state is: every
  prompt field in `mtg-player/src/llm.rs`'s `GAME_RULES` format documented
  and populated; hidden information never in the seat's prompt; legal
  actions listed match what the engine accepts; a schema-constrained answer
  that the engine rejects (prompt/parse mismatch); resume-from-recap
  fidelity after `--save`/`--resume`; retry/fallback behavior when the CLI
  fails (point `CLAUDE_CODE_BIN` at a script that exits non-zero or hangs).
- **R10 Harness subprocess contract**: with `CLAUDE_CODE_BIN` pointing at a
  wrapper that logs argv/stdin and delegates to the real `claude`: session
  ids stable across a game, one subprocess per decision, no leaked
  processes or temp dirs after exit/Ctrl-C/kill, the game never blocks
  past the call timeout, usage totals printed at game end are sane.

## Filing

Per the pipeline contract, one issue per distinct defect, labels
`bug` + `phase:robustness`, title `[robustness] <short symptom>`, body with
**Found-by** (this crew, date, mission), **Repro** (exact commands — a
fresh reader with no context must be able to run them), **Evidence**
(verbatim captures, `/proc` readings, exit codes), and a **Confidence**
line. Before filing, search open issues for the same symptom and comment
there instead of duplicating. UX judgments are fine to file — label the
severity honestly, and say when something might be deliberate.

Never leave a session or temp file behind: `tmux kill-server`, clean
`logs/robustness/` of passing runs.
