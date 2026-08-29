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

Agents may invent missions beyond this menu; log them in the ledger so
they enter the rotation.
