# Nightly playtest crew: personas and mission menu

The nightly playtest routine plays ~30 games of hotseat self-play through
the real CLI (`mtg-runner --p1 cli --p2 cli` in tmux) — the agent plays
BOTH seats, so no random players are involved. Each game is assigned a
persona and a mission. The ledger (`reports/playtests/LEDGER.md`) records
every mission played; a mission from the last two weeks is off the menu
unless re-probing a fresh fix. Findings become GitHub issues per
`docs/plans/bug-pipeline.md` (`phase:playtest`).

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

Agents may invent missions beyond this menu; log them in the ledger so
they enter the rotation.
