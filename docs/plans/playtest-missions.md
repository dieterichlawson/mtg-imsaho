# Nightly playtest crew: personas and mission menu

The nightly playtest routine plays ~30 games a night, most of them hotseat
self-play through the real CLI (`mtg-runner --p1 cli --p2 cli` in tmux)
with the agent at both seats. Each game is assigned a persona and a
mission. Findings become GitHub issues per `docs/plans/bug-pipeline.md`
(`phase:playtest`).

Which missions: the ledger (`reports/playtests/LEDGER.md`) records every
mission ever played, and picking from the menu goes by it. **A mission
that has never been played comes first** — that is what drains the
proposals below. After those, a mission played in the last two weeks is
off the menu unless it is re-probing a fresh fix, and among the rest,
prefer the one played longest ago and a spread of targets across the
night.

The menu covers all three targets named in the pipeline's glossary: the
**engine** (Competitor, Rules Lawyer), the **machine** (Vandal, Operator)
and the **harness** (Handler). Say which one an issue is about in its
**Target** line. The Operator and Handler missions are the only ones that
don't run two `cli` seats; the Handler's `claude-code` seat runs on plan
quota through `claude -p` — a metered `claude`/`gemini` API seat is never
allowed, on any mission.

Game setup: pick deck pairs from `decks/` and `decks/coverage/`, or write
one-off decks into a temp file for a mission (deck files are
`COUNT NAME` lines). Run with `--log logs/playtest/<game>.log` and use
`--save` so anomalies leave a resumable snapshot. Delete logs after the
night's report is written; never commit them.

## Personas

**The Competitor** — plays both seats to win, honestly, like two humans.
Surface-level goals: real strategic lines, combat math, resource
decisions. Reports anything confusing or wrong: misleading prompts,
missing options, log lines that misdescribe what happened, rules results
that contradict the CR.

**The Rules Lawyer** — plays both seats to *maximize rules interaction*
and verifies every step against the CR as it goes. Wins don't matter;
illegal or dubious resolutions do.

**The Vandal** — plays both seats to break the engine. Wins don't matter;
panics, hangs, stuck prompts, corrupted state, and nonsense output do.

**The Operator** — neither plays to win nor tries to break anything: runs
the binary the way an operator would and checks it kept its promises.
Save/resume fidelity, log and screen reconciliation, reproducibility from
the outside, resource behaviour over a long run. The Vandal asks whether
the machine survives abuse; the Operator asks whether it told the truth.

**The Handler** — gives one seat to the LLM interface
(`--p1 claude-code`, `cc` for short) and audits what that seat is *told*
and what the engine does with its answer: the prompt's contents, the
response schema, the subprocess contract. Reads
`mtg-player/src/llm.rs` and `docs/llm-harness.md` before playing.

## Mission menu

Competitor:
- C1 aggro mirror: race, combat tricks, damage ordering
- C2 control mirror: counterspells, instant-speed battles, priority holds
- C3 attrition: removal-heavy, graveyard value, flashback
- C4 tribal synergy: humans/vampires/zombies/spirits deck vs another tribe
- C5 planeswalker-centric: protect and ultimate a walker; attack one down
- C6 equipment voltron vs token swarm
- C7 curses: stack multiple curses on one player and play through them
- C8 transform tempo: werewolf day/night flip manipulation via spell counts
- C9 aristocrats/sacrifice value vs go-wide tokens: trade into sac outlets
  for value, race a token swarm on the other side of the table
- C10 mulligan-to-five resource grind: both seats mulligan to the
  floor, then play the resource-starved game out honestly; watch hand
  sizes, bottoming counts and land-drop accounting
- C11 lifegain vs burn race: set up exact-lethal and exact-survival
  spots deliberately; verify every life transition and that the game
  ends at exactly 0 at the right time (CR 704.5a)
- C12 mill race / winning by decking: race a mill clock against a board
  clock; verify the loss happens on the DRAW from an empty library, not
  when the library empties (CR 104.3c / 704.5b). Needs a pairing that
  can actually mill — WU coverage (Curse of the Bloody Tome, Undead
  Alchemist, Armored Skaab) is the mill seat that works
- C13 flyers vs ground stall: build a ground stall and win in the air;
  chump blocks, evasion checks, combat tricks every turn
- C14 topdeck war: empty both hands by turn ~8 and play a pure topdeck
  game to a conclusion; verify draw counts, hand size and
  discard-to-hand-size, and that no draw is duplicated or skipped
- C15 mana pool and land-drop accounting: float mana and let it empty at
  every step/phase boundary (CR 500.4), verify no mana burn, verify the
  one-land-per-turn rule and that lands are refused off-turn or with a
  non-empty stack (CR 305.1)
- C16 combat trick war across every combat priority window: cast instants
  at beginning of combat, after attackers, after blockers, between
  first-strike and regular damage, and at end of combat; verify priority
  exists at each and that removing a blocker doesn't unblock the
  attacker (CR 509.1h)
- C17 repeatable activated-ability value engines: Moorland Haunt,
  Nephalia Drownyard, Ludevic's Test Subject, Avacynian Priest, equip
  costs; verify every activation actually pays its cost, uses the stack,
  and that counters/state don't drift over a long game
- C18 sweeper vs go-wide: build 3+ creatures a side, then break the board
  with Divine Reckoning; verify each player chooses their own keeper in
  APNAP order (CR 101.4) before any simultaneous sacrifice (CR 701.17),
  and that tokens cease to exist rather than resting in a graveyard
- C19 multi-block combat math: force double and triple blocks every
  combat; verify P/T after anthems, damage marked, who dies, life lost,
  and that the attacking player gets the ordering and assignment choices
  the CR gives them (CR 509.2, 510.1a-d)
- C20 hand attack / discard-based control: win by stripping the hand.
  Targeted vs random vs "you choose" discard, discard from an empty hand,
  cleanup discard-to-hand-size; verify hand-size accounting is exact and
  no hidden information leaks at the other seat's prompt
- C21 land destruction and mana denial: attack the mana base. Verify a
  destroyed land's mana is really gone, no phantom pool, one replacement
  land per turn (CR 305.2), a landless player still gets priority, and
  unpayable costs leave the menu rather than failing after selection
- C22 non-combat damage and life drain attrition: win without combat
  damage. Verify every life transition, "loses life" vs "is dealt
  damage", simultaneous drain triggers ordered by their controller
  (CR 603.3b), lifelink as part of the damage event (CR 702.15a), and
  the game ending at exactly 0 on the next SBA check (CR 704.5a)
- C23 play/draw and opening-procedure fairness: play the same pairing
  twice with the seats swapped; verify the starting player skips their
  first draw (CR 103.7a), the London mulligan counts, summoning sickness
  on turn 1 (CR 302.6), APNAP consistency (CR 101.4), and whether the
  CLI ever says which seat is on the play

Rules Lawyer:
- L1 stack battles: respond to everything; 3+ deep stacks; order triggers
  differently each time a ChooseTriggerOrder prompt appears
- L2 targeting edges: target own permanents with removal, retarget-bait
  with hexproof/protection, fizzle spells deliberately
- L3 optional everything: decline every "may"; verify nothing forces
- L4 combat rules: menace/multi-block, first-strike ordering, trample
  assignment, mid-combat removal of blockers/attackers/walkers
- L5 cost edges: flashback from graveyard, X=0 and X=max, additional
  costs (sacrifice/exile), Snapcaster-granted flashback
- L6 copy/DFC: Evil Twin copies of transformed werewolves, token copies,
  legend-rule keep choices
- L7 zone identity: reanimate, bounce, and re-cast the same card;
  verify new-object rules (counters/attachments/damage gone)
- L8 SBA order: simultaneous deaths, Angelic-Overseer-style dependency,
  both players to 0 life
- L9 replacement effects: stack multiple replacement effects on the same
  event (damage prevention/redirection, enters-with-counters vs a static
  buff); verify the affected player/object's controller chooses the order
  (CR 616) and only one applies per layer of the event
- L10 mana ability edges: tap-for-mana abilities that don't use the stack;
  activate mana abilities in response to a targeted spell/ability to
  verify no missed priority window and correct fizzle/cost-payment timing
- L11 layers (CR 613): stack anthems (7c), +1/+1 counters (7d),
  P/T-setting (7b) and type/ability grants (4/6) on one creature at
  once; verify layer order, timestamps, and that removing one effect
  recomputes rather than un-adding a stale number
- L12 attack/block requirements vs restrictions (CR 506.4, 508.1d,
  509.1c): menace, "can't block", "must attack if able", tapped and
  summoning-sick creatures all live at once; verify the engine
  maximizes satisfied requirements without violating a restriction and
  refuses illegal sets rather than silently trimming or augmenting them
- L13 leaves-the-battlefield and exile-and-return ordering (CR 603.6d,
  603.10, 400.7): Fiend Hunter as the centerpiece — exile a creature,
  then kill or bounce the Hunter, including in response to its own ETB
  trigger; verify the returning creature is a new object. Wants a deck
  pair with instant-speed removal that can kill a 1/3
- L14 timing and priority enforcement (CR 305.1, 307.1, 606.3, 117):
  probe for any land played off-turn or with a non-empty stack, any
  sorcery-speed spell offered at instant speed, any loyalty ability
  outside its window or twice per turn, any skipped or doubled priority
- L15 attachment legality and SBAs (CR 704.5m/n/p, 303.4): attach auras
  and equipment, then make the attachment illegal (kill, bounce, grant
  protection/hexproof, change type); verify auras go to their OWNER's
  graveyard while equipment merely unattaches. Wants a deck pair with a
  real protection/hexproof granter
- L16 copy effects (CR 706): Cackling Counterpart, Evil Twin, Essence of
  the Wild; verify only copiable values are copied (no counters, auras,
  damage or tap state), Evil Twin's name/ability exception, the legend
  rule on a copied legend, and flashback exile on resolution
- L17 morbid (ability word, checked on resolution): Brimstone Volley,
  Morkrut Banshee, Festerhide Boar; kill a creature in response and
  verify the condition is re-checked as the spell/trigger resolves, that
  tokens dying count, and that bounce/exile/discard do not (CR 700.4)
- L18 token existence (CR 111.7, 704.5e) and token-doubling replacement
  effects (CR 614/616): Spider Spawning, Parallel Lives, Kessig
  Cagebreakers; verify dead tokens leave no graveyard residue, aren't
  counted as creature cards, and that doubling applies once per event
  and only to its controller's tokens
- L19 Curses and "Enchant player" legality (CR 303.4a, 702.5, 704.5m):
  verify only players are offered as targets, that a curse may be cast on
  yourself, Curse of Death's Hold's layer-7c -1/-1 with SBA deaths and
  recompute-on-removal, and Curse of the Nightly Hunt's attack
  requirement (CR 508.1d)
- L20 evasion and blocking legality (CR 509.1a-c, 702.9/702.11/702.16):
  Invisible Stalker's "can't be blocked", Blazing Torch's conditional
  evasion, Vampire Interloper's "can't block", Crossway Vampire's
  one-turn restriction, flying vs reach, and hexproof being targetable
  by its own controller but not the opponent
- L21 illegal targets on resolution (CR 608.2b, 603.3d, 601.2c): make a
  target illegal after the spell or trigger is on the stack (kill,
  bounce, exile, hexproof, protection, type or controller change).
  Verify all-targets-illegal is countered on resolution with no partial
  effects, some-targets-legal still does as much as it can, targets are
  locked in at announcement, legality is re-checked on resolution, and
  a fizzle is reported differently from a normal resolution
- L22 cost legality and payment (CR 601.2f-h, 117.4, 118.4, 118.6): an
  unpayable additional cost must make the spell un-castable and absent
  from the menu; sacrifice costs pay on activation and only from
  permanents you control; life payment can't exceed your life total;
  mana is deducted exactly and never spent on the wrong spell; and no
  prompt may let you un-pay a cost already paid
- L23 regeneration, indestructible and "destroy" replacement (CR 701.15,
  702.12, 615, 704.5g): a shield taps, removes from combat, clears
  damage and is used up; a second destruction the same turn kills; no
  save from sacrifice, exile or a 0-toughness SBA; indestructible
  ignores lethal damage and "destroy" but still dies to 0 toughness
- L24 turn structure and trigger windows (CR 500-514): no priority in
  untap (CR 502.3), upkeep triggers before the draw, the draw happens
  before priority (CR 504.1), an end-step trigger created during the end
  step waits for the next turn (CR 513.2), and a cleanup with a discard
  or a trigger grants priority and a second cleanup step (CR 514.3a)
- L25 hidden-information integrity (CR 400.2, 701.15, 701.18, 103.1) in
  a shared-terminal hotseat: every pane (battlefield, i, d, g, e, /)
  scoped to the prompting seat; "reveal" shown to both and "look at"
  only to the chooser and never echoed into the shared log; library
  order not leaked; face-down exile stays hidden

Vandal:
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

Operator:
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

Handler:
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

## Adding a mission

Any agent may add one — the crew mid-night, the fixer after a fix, anyone
reading the engine who notices a hole. There is no separate queue and no
approval step: a new mission goes straight onto the list above, under the
persona whose mindset it needs, with the next free id in that family and a
provenance tag that stays until it is first played:

```
- H5 [proposed 2026-09-04, from #147] hidden information in the resumed
  seat's recap: ...
```

Because never-played missions are picked first, adding one is the same as
scheduling it. Four rules keep the menu worth reading:

1. **Write a mission, not a topic.** Say what to set up, what to do, and
   what to verify, in the voice of the entries above. "Check trample
   more" schedules nothing.
2. **Cite what prompted it** — an issue number, a ledger row, a CR rule,
   the file you were reading when you noticed. A mission nobody can trace
   back to an observation is a guess.
3. **Extend before you add.** If it is a wrinkle on an existing mission,
   add the wrinkle to that entry; a menu of 80 near-duplicates rotates
   worse than one of 40 distinct ones. Search the list first.
4. **Commit it on its own**, so the addition is reviewable as a change.

Dropping a mission is the same move in reverse: if a mission has been
played several times and never found anything, say so in the commit and
remove it. The menu is meant to churn.
