# Playtesting the game

Subject: the rules engine — whether a game of Magic played through this
program follows the Comprehensive Rules. Two `cli` seats in tmux, you at
both of them, playing real games with real decks.

This is the biggest subject and the one with the most surface. Almost
every engine bug ever found here was found by playing a game and
noticing that something was wrong, not by testing a function.

## Before you start

The ideas below are a starting point, not a syllabus. They are what
previous nights happened to think of, and the bugs that mattered most
were usually not on the list when the night began. The real method is the
one underneath them: read the code that implements this, read the rule or
the contract it is supposed to satisfy, and find where the two disagree.
When you find a way to look that the list doesn't have, take it — and
then add it, per "Adding an idea" in `docs/playtest/README.md`.

## Where to look

- `mtg-engine/src/` is the engine: `engine.rs` (priority, casting, action
  generation), `stack.rs`, `combat.rs`, `damage.rs`, `sba.rs`,
  `replacement.rs`, `triggers/`, `state.rs`, and `cards/` for individual
  card behaviour. Reading a card's implementation before playing it is
  the fastest way to know what to try.
- The Comprehensive Rules. There is no copy in the repo — cite rule
  numbers in issues so a fixer can look them up. The sections that pay
  off most here are 100s (game concepts), 300s (card types), 400s
  (zones), 500s (turn structure), 600s (spells and abilities), 700s
  (additional rules) and 702 (keywords).
- `mtg-engine/src/invariants/` is what the fuzzer already checks
  card-independently at every decision point. If a property is in there,
  the nightly fuzz is already hunting it over ~110k games and you
  probably shouldn't spend a night on it by hand. What you can do that
  the fuzzer can't is judge a specific card's behaviour.
- `mtg-engine/tests/` for what is already pinned, and closed issues for
  what has already been found.

## Ideas

Two lenses have historically been productive here.

**The Competitor** plays both seats to win, honestly, like two humans.
Real strategic lines, combat math, resource decisions — and reports
anything confusing or wrong along the way: misleading prompts, missing
options, log lines that misdescribe what happened, results that
contradict the CR.

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
- C24 planeswalkers as a combat target: attack a walker, defend it, and
  kill it. Verify the attack target is chosen per attacker at declare
  attackers and the CLI offers it per attacker (CR 508.1a), that combat
  damage removes that many loyalty counters (CR 306.7, 120.3c) and stays
  removed rather than clearing at cleanup, that 0 loyalty sends it to its
  owner's graveyard at the next SBA check (CR 704.5i), that a trampler
  measures lethal in loyalty before spilling to the controller (CR
  702.19b), that a burn spell reaches a walker only if it says "any
  target" (CR 115.4 — there is no redirection any more), and that loyalty
  abilities are sorcery-speed and once per turn per permanent (CR 606.3,
  118.5). Liliana of the Veil and Garruk Relentless are the only
  implemented walkers and both are 1-ofs, so write a one-off deck

**The Rules Lawyer** plays both seats to *maximize rules interaction* and
verifies every step against the CR as it goes. Wins don't matter;
illegal or dubious resolutions do.

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
- L26 [proposed 2026-09-04, from the C24 night and #219/#220] planeswalker
  combat leftovers (CR 506.4c, 508.1a, 510.5, 702.19b): send TWO attackers at
  once, one at the player and one at a walker, and verify the blocker prompt
  and the damage split are right for each; kill or bounce the attacked walker
  after blockers are declared and verify the attacker deals no damage at all
  rather than falling through to the player; block a trampler that is
  attacking a walker and verify the excess crosses the blocker to the walker
  (lethal = loyalty) and only then to the player; attack a walker with a
  first- or double-striker and verify the regular damage step does nothing
  once the walker died in the first-strike step. Wants Liliana of the Veil
  (br-coverage) or Garruk Relentless (ug-coverage) plus Kessig Wolf Run for
  the trample grant; write a one-off deck, the 1-ofs are unreachable
- L27 the graveyard as an ordered, shared, contested zone: reanimate or steal
  an opponent's creature (Grimoire of the Dead, Olivia Voldaren, Traitorous
  Blood), attach one player's Aura and the other's Equipment to it, then kill
  it — verify the creature goes to its OWNER's graveyard (CR 404.3), each Aura
  to ITS owner's (CR 704.5m), and the Equipment merely unattaches (CR 704.5n),
  all from one death. Sweep several creatures at once and verify every "dies"
  trigger sees the others having died (CR 603.10a) and that tokens leave no
  residue (CR 111.7). Cast a flashback card from the graveyard and make it
  resolve, be countered, and fizzle — all three must exile it (CR 702.34a),
  never return it. Finally check the ORDER: put three cards into one graveyard
  on three known turns and compare `g` against arrival order (CR 404.2) — the
  engine keeps no graveyard order at all (#222), so this is a re-probe until a
  real ordered zone exists. Needs reanimation or theft, Auras, Equipment,
  flashback and a sweeper; no coverage pairing has all of these, so write
  one-off decks
