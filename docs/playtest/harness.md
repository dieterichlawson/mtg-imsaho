# Playtesting the harness

Subject: the LLM interface — the prompts, the response schema and the
conversation an LLM seat plays a game through. Not the CLI a human
drives, and not the rules: this is about whether a model sitting in a
seat is told what it needs, offered what it may do, and understood when
it answers.

This is the newest subject and the thinnest. Almost nothing here has
been played.

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
