# The bug pipeline: who finds, who files, who fixes

Every automated testing phase reports bugs as GitHub issues with uniform
metadata; one fixer loop works the queue and merges. Finders never fix;
the fixer never hunts.

Three words that are easy to confuse, fixed here:

- **the engine** — the rules (`mtg-engine`).
- **the machine** — the binaries as programs: the CLI/TUI, flags, files,
  signals, save/resume (`mtg-runner`, `mtg-player`'s interactive surface).
- **the harness** — the LLM interface: the prompts, the response schema and
  the conversation an LLM seat plays a game through (`mtg-player/src/llm.rs`
  and its backends). Documented in `docs/llm-harness.md`.

## Finders (each files issues, labeled with its phase)

| Phase | What it runs | Label | Filed by |
|---|---|---|---|
| Nightly fuzz | `nightly-fuzz` workflow (~110k seeded games) | `phase:fuzz` | the workflow (per failing seed) |
| Weekly mutants | `weekly-mutants` workflow (~2,365 engine-core mutants) | `phase:mutants` | the workflow (new survivors beyond `reports/mutants-accepted.txt`) |
| Nightly playtest | "Nightly playtest crew" routine (LLM self-play via the CLI) | `phase:playtest` | the routine |
| Nightly robustness | "Nightly robustness crew" routine (`prompts/ROBUSTNESS_CREW_PROMPT.md`: audits the binaries as programs through tmux — signals, TTY edges, input abuse, files/flags, save/resume, log hygiene, endurance, determinism, and the two missions that poke the LLM harness's prompt protocol and subprocess contract; seats are `cli`/`random`/`claude-code` — the LLM seat through `claude -p` on plan quota — never metered `claude`/`gemini` API seats) | `phase:robustness` | the routine |

All issues also carry the `bug` label. Labels are auto-created on first
use, so a new phase just picks a `phase:<name>` label and starts filing.

## Issue format

- **Title**: `[<phase>] <short symptom>` — for fuzz, include the pair and
  seed (`[fuzz] wu-coverage-vs-br-coverage seed 1234: <violation>`), which
  doubles as the dedupe key.
- **Body** must contain:
  - `**Found-by**:` phase, date, and the run that produced it (workflow
    run id / routine session / ledger row).
  - `**Repro**:` the exact command(s). For fuzz: the seeded runner
    invocation. For mutants: the `cargo mutants -F` re-run. For playtest:
    the game setup plus the decision sequence (and a `--save` snapshot
    path or game-log excerpt when the line isn't seed-reproducible).
  - `**Evidence**:` invariant output / log excerpt / observed vs expected
    with the CR rule cited when known.
- **Dedupe**: before filing, search open issues for the same
  signature (fuzz: pair+seed in title; playtest: same symptom); comment on
  the existing issue instead of filing a duplicate.

## The fixer ("Daily bug fixer" routine)

Runs daily after the finders. Takes open `phase:*` issues (oldest first,
grouping obvious duplicates), and for each: reproduce → root-cause →
fix the mechanism, wherever it lives (never a per-card special case) → regression
test (mutation-checked where feasible) → full workspace suite green →
**merge to master** → close the issue citing the commit. Issues it cannot
reproduce or safely fix get a comment with the diagnosis and stay open
for a human. Duplicate issues are closed as duplicates of the one that
carries the fix.

The fixer is the only automated writer to master. Finders write only
issues, plus (for playtest) ledger/report docs.
