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
- C25 the graveyard as a live stat line: play a Boneyard Wurm /
  Splinterfright / Lumberknot / Wreath of Geists deck into a graveyard-hate
  deck (Purify the Grave, Ghoulcaller's Bell, Graveyard Shovel, Sever the
  Bloodline) and verify the characteristic-defining P/T (CR 604.3, layer 7a)
  recomputes in every zone and at every instant: change the graveyard during
  declare-blockers and check power BEFORE damage (CR 510.1a), exile creature
  cards and check the creature dies at the next SBA check when toughness hits
  0 (CR 704.5f), and stack an anthem and a counter on top to check 7a→7c→7d
  order. No coverage pairing has both halves; write one-off decks
- C26 alternative costs and cost reduction: Rooftop Storm ("cast Zombie
  creature spells without paying their mana costs", CR 118.9) and Heartless
  Summoning ({2} less, and -1/-1 that kills your own X/1s). Verify the free
  cast is offered for Zombie CREATURE spells only, that both the paid and the
  free option appear when you can afford either (CR 601.2b), that a reduction
  never eats a coloured symbol (CR 601.2f), that an additional cost survives
  an alternative cost, and that killing the cost source with the spell on the
  stack does not rebill it (CR 601.2h)
- C27 the nonbasic mana base: the five check lands (Isolated Chapel, Clifftop
  Retreat, Woodland Cemetery, Hinterland Harbor, Sulfur Falls) enter tapped or
  untapped as a replacement checked AS they enter (CR 614.1c) and are never
  re-evaluated later; Ghost Quarter's search belongs to the DESTROYED land's
  controller, is optional, finds only basics, enters untapped and does not eat
  their land drop (CR 701.19, 305.2); the utility lands (Gavony Township,
  Kessig Wolf Run, Moorland Haunt, Nephalia Drownyard, Stensia Bloodhall) are
  NOT mana abilities and must use the stack and grant a priority window
  (CR 605.1a). The two real games reached almost none of these on their own —
  budget a targeted probe deck for the utility lands
- C28 the artifact deck and the artifact hate: play the Equipment deck as an
  artifact deck (Silver-Inlaid Dagger, Butcher's Cleaver, Demonmail Hauberk,
  Runechanter's Pike, Inquisitor's Flail, Mask of Avacyn, Wooden Stake,
  Blazing Torch, Manor Gargoyle, Galvanic Juggernaut) into Ancient Grudge,
  Naturalize and Stony Silence. Verify equip is sorcery-speed and targets only
  your own creature (CR 702.6b), moving an Equipment (702.6c), Equipment
  unattaching rather than dying (704.5n), Stony Silence stopping equip and
  artifact-creature abilities but not land mana, and an artifact creature
  answering to both creature and artifact removal. Demonmail Hauberk's
  "Equip—Sacrifice a creature", Runechanter's Pike, Inquisitor's Flail and all
  four Ancient Grudge modes went undrawn in two games — stack them higher
- C29 X spells and variable damage: Devil's Play ({X}{R}, flashback {X}{R}{R}{R}
  — a SECOND independently chosen X), Harvest Pyre (X paid by exiling from your
  own graveyard), Heretic's Punishment, Blasphemous Act (a cost that varies with
  the board). Verify X is chosen at announcement and locked (CR 601.2b), X=0 is
  legal and deals 0, X is 0 in every zone but the stack (CR 202.3b — check a
  milled Devil's Play's mana value), the exile cost is paid at announcement
  (601.2h), and Blasphemous Act re-prices as the board changes and never falls
  below {R}. Its 8-creature floor needs a real go-wide board to reach
- C30 [tried 2026-09-06; no engine bug found, see #257] tempo as pseudo-removal:
  play a real WU bounce/tap/pacify deck (Silent Departure + flashback, Feeling of
  Dread, Claustrophobia, Grasp of Phantoms, Hysterical Blindness, Avacynian
  Priest, Rebuke, Ghostly Possession) against a creature deck, and touch the
  opponent's board at every point in combat: tap before attackers (CR 508.1a) vs
  after (508.1e/506.4), tap or bounce a declared blocker (509.1h/506.4c), bounce
  an enchanted creature (704.5m + 400.7), and hold Claustrophobia over several
  untap steps. All of that held over four games and ~1500 actions. Unreached:
  Grasp of Phantoms (never drawn in four games) and a bounce of a creature
  enchanted at that moment — start there
- C31 [tried 2026-09-06 → #301] card-advantage engines and exact draw accounting:
  Think Twice, Desperate Ravings, Forbidden Alchemy, Divination, Bloodgift Demon,
  Curiosity, Sturmgeist, Murder of Crows, Mindshrieker. Win on cards against a
  real aggro clock and reconcile library/hand/graveyard/exile by hand after EVERY
  draw effect. Verify Desperate Ravings draws both cards before discarding
  exactly one at random from the full post-draw hand (cast it 4+ times and record
  the choices — the discard is correct and is logged nowhere, #301), that
  Forbidden Alchemy moves exactly 4/1/3 and keeps all four secret (CR 400.2,
  701.18a), that Bloodgift Demon can target either player (115.4) with the target
  chosen on the stack (603.3d), that Curiosity fires on damage to a PLAYER and
  not to a creature, that Mindshrieker's +X/+X is the milled card's mana value
  and stacks (layer 7c), and that flashback exiles on resolution (702.34a). The
  coverage decks are far too thin — write a one-off UBr engine deck. Murder of
  Crows' body and a 12+ card cleanup discard went unreached in five games
- C32 [tried 2026-09-06; every CR claim held, see #243] lifelink and life as a
  resource — the gap C22 recorded as "lifelink untestable". Race a burn deck with
  Markov Patrician, Butcher's Cleaver, Disciple of Griselbrand, Gnaw to the Bone
  and Paraselene, and reconcile every life transition against the log. Verify
  lifelink is part of the damage event and not a trigger — no stack object, life
  gained in the same event as the damage, including for a lifelinker that dies to
  that same combat damage (CR 702.15a, 510.2) — and that damage to a creature
  counts. Butcher's Cleaver is the sharpest probe: equip it to a non-Human and
  then to a Human and compare, and reach a type change (Moonmist on a Human
  Werewolf) to see the grant drop and come back. Then close on life as a clock:
  prevented damage gains nothing, lifegain in response to lethal saves you, and
  the game ends at exactly 0 on the next SBA check (704.5a). Unreached: a
  lifelinker blocked by MULTIPLE creatures — every lifelinker in the pool has 1-3
  toughness, so a rational defender never gang-blocks
- C33 [tried 2026-09-06 → #300] self-mill as a resource engine (distinct from
  C12, which raced an OPPONENT's library to zero): the graveyard is the payoff
  and running yourself low is the risk. Mulch, Dream Twist, Armored Skaab,
  Splinterfright, Boneyard Wurm, Skaab Ruinator (cast from the graveyard by
  exiling three creature cards), Memory's Journey, Past in Flames and Spider
  Spawning against a real clock. Verify the additional cost is paid at
  announcement and the card leaves the menu when it cannot be paid (CR 601.2f,
  601.2h), that graveyard-count P/T recomputes live — including mid-combat, after
  blockers (604.3) — that every mill moves exactly N from the top and the zone
  totals still sum to the deck, and that milling your library to zero does NOT
  lose: the loss is on the next DRAW (104.3c, 704.5b). A deliberately small
  (~32-card) probe deck reaches library 0 in a handful of turns where a 60 never
  will. Unreached: Moldgraf Monstrosity's death trigger and a Past in Flames into
  Spider Spawning turn
- C34 [tried 2026-09-06 → #302] the defensive deck and the life-total exchange:
  Tree of Redemption ("{T}: Exchange your life total with this creature's
  toughness") behind Grave Bramble, Manor Gargoyle, Somberwald Spider and
  One-Eyed Scarecrow, with Gavony Township as the slow win condition, against a
  real aggro deck that will attack into it. The exchange is a layer-7b set (CR
  613.4b): verify the value handed to the player is the MODIFIED toughness, that
  anthems and +1/+1 counters re-apply on top of the set value (7b→7c→7d), that
  marked damage is neither laundered nor mistaken for toughness, and that a Tree
  whose new toughness is at or below its marked damage dies at the next SBA check
  (704.5g). All of that held; what did not is that the ability writes the
  object's BASE toughness instead of a 7b effect (#302) — the copy and
  zone-change readers are patched by hand, so re-probe any NEW reader of base
  P/T. Defender is absent from the eligible-attackers list rather than
  offered-and-rejected (702.3b), and Manor Gargoyle's indestructible correctly
  switches off while its {1} has removed defender. Unreached: a second
  P/T-setting effect on the Tree (ISD has none), and exchanging to exactly 0 life
  — a 0-toughness Tree dies to 704.5f before it can be tapped

- C35 [tried 2026-09-07 → #329, #331] the token deck vs the sweeper — the token
  as an object under anthems, doubling and mass removal. Write one-off decks (no
  coverage pairing has both halves): a go-wide side of Midnight Haunting, Moan of
  the Unhallowed, Army of the Damned, Doomed Traveler, Mausoleum Guard, Gutter
  Grime and Walking Corpse under Parallel Lives, Intangible Virtue, Glorious
  Anthem and Gavony Township, against Divine Reckoning, Blasphemous Act, Rolling
  Temblor, Sever the Bloodline, Curse of Death's Hold and Tribute to Hunger.
  Verify Parallel Lives replaces the creation event exactly once and only for its
  own controller (CR 614.1b — cast the SAME card from both seats with one
  Parallel Lives out and compare 4 tokens to 2), that 7c anthems stack on a token
  and 7d Township counters go on top of them, and that a nontoken creature gets
  the anthems but not Intangible Virtue. The sharpest reachable states: a Curse
  of Death's Hold that turns every 1/1 token into a 0/0 dying on arrival (614.1b
  → 7c → 704.5f in one resolution), a graveyard that has swallowed six dead
  tokens and one creature card so Spider Spawning must count exactly ONE (111.7 —
  tokens are not creature cards), and Army of the Damned doubled to 26 Zombie
  tokens so Blasphemous Act prices at exactly {R} and Sever the Bloodline exiles
  all 26 with no exile residue. Note that Sever the Bloodline's Oracle text is
  "and all other creatures with the same name as that creature", with NO "its
  controller controls" clause, and the engine's cross-controller exile is
  correct. Every CR claim above held; the defects are the log (#329: only Kessig
  Cagebreakers and Spider Spawning got #92's fix, so Army of the Damned still
  says "13" when 26 enter) plus token names carrying a " Token" suffix CR 111.4
  does not give them (#331). Unreached: Kessig Cagebreakers' tapped-and-attacking
  Wolves under a doubler, Endless Ranks of the Dead's upkeep count, Elder of
  Laurels, and a second Parallel Lives for the 4x case — stack those higher
- C36 [tried 2026-09-07; no engine bug found] the +1/+1 counter as an
  accumulating clock — combat-damage triggers over a long game. Play the vampire
  shell (Stromkirk Noble, Bloodcrazed Neonate, Rakish Heir, Falkenrath Marauders,
  Curse of Stalked Prey, Bloodline Keeper, Vampiric Fury) against a deck that
  blocks and removes, and reconcile every counter by hand against `--log` each
  combat. Verify one trigger per combat damage EVENT and never per point (CR
  510.2 — a 7-power hit gives one Heir trigger, not seven); that stacking three
  or four triggers on one attacker is ONE CR 603.3b ordering prompt, not two
  groups, and that when the Curse's controller differs from the Heir's the active
  player's triggers go on the stack first (CR 101.4) — the only route is the
  opponent casting Curse of Stalked Prey on THEMSELVES, which is legal and
  reaches the split in one turn; that combat damage to a blocking creature and to
  a planeswalker fire nothing at all (CR 508.1a) while trample excess reaching
  the player DOES fire it (CR 702.19b) even when the trampler dies in the same
  step and CR 121.1 then drops the counter; that a first-striker deals damage
  once and gets one set of counters (CR 510.4/510.5); and that counters survive
  cleanup, tap/untap and Vampiric Fury expiring while vanishing on a zone change
  (a Marauders with 8 counters reads 2/2 in exile, CR 400.7/111.2). Bloodline
  Keeper's transform is correctly absent at four Vampires and offered at five,
  and layer 7c/7d stacked right (2/2 base + counters + Lord of Lineage anthem +
  Vampiric Fury = 9/7). `decks/rb-vampires.txt` is adequate but too thin on
  Rakish Heir and the Curse to reach the stacking cases reliably — write one-offs.
  Unreached: a double-striker's two damage steps (no double strike in these decks
  — needs Terror of Kruin Pass), a counter-carrying creature dying and being
  reanimated as a new object, and a Rakish Heir killed by first-strike damage
  before the regular damage step
- C37 [tried 2026-09-07 → #328] creature TYPE as a live resource: make every
  Human-conditional in the pool re-read itself while the types move under it
  (CR 613.1d layer 4 feeding layers 6 and 7c). Three one-off deck pairs (Angelic
  Overseer / Cloistered Youth / Villagers of Estwald / Mayor of Avabruck /
  Butcher's Cleaver / Bonds of Faith / Moonmist / Village Bell-Ringer against
  Geistflame / Brimstone Volley / Smite the Monstrous / Slayer of the Wicked /
  Elite Inquisitor). Everything held over three games: ONE Bonds of Faith on a
  werewolf DFC switched between its +2/+2 half and its can't-attack-or-block half
  on each transform with no re-target and no new object (the Werewolf face was
  ABSENT from the attacker list, not offered-and-rejected, 509.1b/508.1a);
  Howlpack Alpha's Werewolf/Wolf anthem dropped a Wolf token 3/3→2/2 the instant
  it flipped back; Butcher's Cleaver lost lifelink in the same window as an
  instant-speed Moonmist (Cloistered Youth 4/1 lifelink → Unholy Fiend 6/3 none)
  and never granted it to an Angel; Angelic Overseer's grant appeared mid-stack
  when a flash Human (Village Bell-Ringer) resolved and fizzled the Smite the
  Monstrous already targeting it (608.2b), removed it from Geistflame's and
  Smite's target lists while it was up, and vanished at the very SBA that killed
  the last Human; Slayer of the Wicked and Smite the Monstrous both read the
  CURRENT type/power. Two corrections for whoever takes this next: Furor of the
  Bitten is NOT a type changer — its Oracle text is "+2/+2 and attacks each
  combat if able" — and every ISD werewolf front face is already "Human
  Werewolf", so there is no printed-vs-current delta for Slayer or Elite
  Inquisitor to get wrong; the only clean Human→non-Human flip in the pool is
  Cloistered Youth → Unholy Fiend (Horror). Unreached: lethal damage MARKED on
  the Overseer while indestructible and then losing the last Human in the same
  window (needs a 4-power ground attacker for the Angel to block — Night
  Revelers, never drawn); Night Revelers' own "haste as long as an opponent
  controls a Human"; the Overseer and a werewolf DFC on the battlefield together;
  and CR 613.7 dependency, which looks unreachable — ISD's other type effects
  (Olivia Voldaren, Grimoire of the Dead) only ADD a subtype and are off-colour
- C38 [tried 2026-09-08 → #326 comment; every CR claim held] the upkeep-trigger
  engine as a clock: give BOTH decks upkeep triggers so one upkeep holds triggers
  from both players, and check CR 603.3b/603.3d, 603.4 and 603.2 on the same step.
  Two one-off pairs (Endless Ranks of the Dead / Bloodgift Demon / Ghoulraiser /
  Altar's Reap against Mayor of Avabruck / Gatstaf Shepherd / Villagers of
  Estwald / Splinterfright, then the same black deck against Curse of the Bloody
  Tome / Delver of Secrets). AP's triggers went on the stack first and resolved
  last, each player ordered only their own group, Bloodgift Demon chose its target
  on the way onto the stack (self-target offered) and still drew and drained after
  Altar's Reap sacrificed the Demon in response, and a dead permanent contributed
  no trigger to the next upkeep. Two corrections: Endless Ranks of the Dead's
  current Oracle has NO "if you control two or more Zombies" clause, so that probe
  reduces to X-counted-on-resolution (0/1/1/2 tokens at 1/2/3/4 Zombies); and the
  ISD curses read "at the beginning of ENCHANTED PLAYER's upkeep", which makes
  Curse of the Bloody Tome / Curse of Oblivion the cheapest way to hand the
  NON-active player a trigger on the active player's upkeep. Unreached: three or
  more distinct AP triggers in one group, Angel of Flight Alabaster's targeted
  upkeep trigger losing its target in response (608.2b), and an upkeep trigger
  from a permanent that changes controller between trigger and resolution
- C39 [tried 2026-09-08 → #362, #363; every CR claim held] vigilance and haste as
  a tempo axis: an aggro deck that attacks every turn while keeping blockers up
  (Elite Inquisitor, Abbey Griffin, Thraben Sentry, Geist-Honored Monk, Intangible
  Virtue on tokens) against one that lands threats and swings the same turn (Manor
  Skeleton, Night Revelers, Falkenrath Marauders, Traitorous Blood). A vigilance
  attacker is untapped and in the eligible-BLOCKERS list on the opponent's turn
  (508.1f); one tapped by an Avacynian Priest is ABSENT from the attacker list
  rather than offered (508.1a); summoning sickness gates both attacking and a {T}
  ability while haste — printed, conditional (Night Revelers' Human check) or
  granted (Traitorous Blood on a Tree of Redemption, whose {T} life-exchange became
  activatable the turn it changed control) — lifts both (302.6); Intangible Virtue
  reached tokens only; Falkenrath Marauders took exactly two counters per combat
  damage EVENT. Best unplanned state: a Thraben Sentry that attacked with vigilance
  and transformed into Thraben Militia mid-combat stayed untapped and stayed an
  attacker while its already-transformed sibling was tapped. Unreached: Skirsdag
  High Priest's morbid {T} on the turn it enters (morbid masks the sickness check —
  set up a death first), and a haste grant that ENDS mid-turn on a creature that
  already attacked
- C40 [tried 2026-09-08 → #357; every CR claim held] intimidate as an evasion clock
  (CR 702.13a): push damage through a board that ought to block. One-off decks —
  Spectral Rider (W), Brain Weevil (B), Gruesome Deformity on a Galvanic Juggernaut
  (the only colourless attacker in the pool) and Gatstaf Shepherd, whose back face
  Gatstaf Howler is green by COLOUR INDICATOR (CR 204.2) and not by a cost —
  against blockers of every class: Walking Corpse / Doomed Traveler / Savannah
  Lions (sharing), Grizzly Bears (not sharing), One-Eyed Scarecrow and Creepy Doll
  (artifact creatures; Equipment does not count), Geist of Saint Traft (the pool's
  only multicoloured creature) and Vampire Interloper, which must be ABSENT from
  the list under 509.1b even though it is black facing a black attacker. All of
  that passed, as did 105.2c (a colourless attacker is blockable only by artifacts)
  and multi-block plus the 509.2 order prompt. What broke is that no pane prints a
  permanent's colour, so the Howler's is unobtainable (#357). Unreachable and still
  open: granting intimidate AFTER blockers are declared (509.1h) — Gruesome
  Deformity is a sorcery-speed Aura and ISD has no flash grant — and any mid-combat
  colour change, which ISD cannot produce

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
- L26 planeswalker
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
- L28 change-of-control effects (CR 613.1b layer 2, 506.4d, 302.6, 404.3,
  611.2b): Olivia Voldaren's {3}{B}{B}, Traitorous Blood and Grimoire of the
  Dead are the only three ways in, and layer 2 had never been exercised before
  2026-09-05. Steal a creature under an anthem and verify it loses the bonus
  (613.1b); verify summoning sickness under the new controller; steal an
  attacker AND a blocker after declarations and verify each leaves combat
  while the attacker stays blocked (506.4d, 509.1h); kill the stolen creature
  and verify it reaches its OWNER's graveyard (404.3); let Traitorous Blood
  expire at cleanup and kill Olivia mid-steal (611.2b). Stack two control
  effects of different durations on one creature — that is where #253 lives.
  No coverage pairing has these; write one-off decks
- L29 static prohibitions — "can't be cast", "can't be activated", "can't be
  targeted" (CR 101.2, 601.2, 605.1a, 702.11e): Nevermore's named card must be
  ABSENT from the menu, not offered-then-rejected, including its flashback, and
  must come back the instant Nevermore dies; the name is chosen as it enters
  (614.12), and may be a card in neither deck. Stony Silence must kill equip and
  artifact mana abilities but not land mana abilities and not triggered
  abilities. Witchbane Orb must remove its controller from opponents' target
  lists entirely — a spell whose only target is that player must vanish from the
  menu — while self-targeting stays legal, and its ETB destroys only the Curses
  attached to its controller. Every rule here passed on 2026-09-05; the defects
  were in how the CLI presents them (#254, #255)
- L30 alternate win and loss conditions at the empty library (CR 104.2b,
  104.3c, 614, 704.5b, 121.3): Laboratory Maniac replaces the draw, so with an
  empty library the draw must WIN immediately as a replacement — not on an SBA,
  not at the next priority. Kill the Maniac first and verify the ordinary
  704.5b loss instead; draw TWO from a one-card and a zero-card library and
  verify draws are sequential with exactly one replacement; verify the Maniac
  never fires for the opponent's empty draw. The runner enforces no minimum
  deck size, so a 14-card deck empties by turn 9 — build one
- L31 the sacrifice family and who does the choosing (CR 701.17, 601.2h,
  700.2, 603.10a, 115.7, 404.3): a sacrifice paid as a COST happens at
  announcement, so countering Altar's Reap does not give the creature back and
  its dies-trigger resolves first; only permanents you CONTROL are in the
  picker (a stolen creature IS); an unpayable sacrifice cost leaves the menu;
  "target player sacrifices" (Tribute to Hunger) prompts THAT seat and the
  caster never sees it; an ability whose cost is sacrificing its own source
  still resolves; a sweeper's simultaneous deaths must all see each other
  (603.10a); and sacrifice beats regeneration and indestructible alike
  (701.17c). Every rule passed on 2026-09-05 — the defect was the log (#263)
- L32 [tried 2026-09-06 → #285, #286] redundant and stacked control effects
  (CR 613.1b, 613.7a, 611.2b, 110.2a): put two control-changing effects with
  different durations on ONE creature. Traitorous Blood THEN Olivia's {3}{B}{B}
  is correct and survives cleanup (#253's fix holds); the REVERSE order — Olivia
  steals it, you take it back with Traitorous Blood, then kill Olivia — hands the
  creature to the thief permanently at your own cleanup, because the revert
  snapshots a controller instead of deriving one from the effects still in force
  (#285). Re-probe both orders after that fix, and note that a control effect
  whose controller equals its original controller now aborts the run under
  `--check-invariants` (#286). Still unreached: two DURABLE effects contesting one
  permanent — one Olivia is reachable, two is not. Wants a one-off Olivia /
  Traitorous Blood deck
- L33 [tried 2026-09-06 → #292; both halves PASSED] the walker-combat cases L26
  could not reach, and a correction: **Lost in the Mist** ({3}{U}{U}, "Counter
  target spell. Return target permanent to its owner's hand") takes
  `TargetFilter::Any`, so a planeswalker IS in its target list — the guide's old
  claim that no implemented card returns a walker to hand was wrong, and the
  walker's own controller can supply the spell it needs. Bounce an attacked
  walker after blockers and between the two damage steps (the attacker must deal
  nothing, and nothing may fall through to the player), and attack a walker with
  Terror of Kruin Pass (Kruin Outlaw's back face, needs a spell-free turn to
  transform) so it dies in the first-strike step — the regular step must then do
  nothing at all (CR 510.4/510.5). All of that held on 2026-09-06 and #246's fix
  holds on the first-strike path. Still unreached: a BLOCKED attacker aimed at a
  walker on the first-strike path, and Terror's menace grant
- L34 [tried 2026-09-06 → #298, #299; every CR claim held] "enters with counters"
  as a replacement effect, not a trigger (CR 614.1c, 614.12, 616.1, 603.6b,
  704.5f): cast Mikaeus, the Lunarch for X=0 and Unbreathing Horde with no other
  Zombie, and verify each is a real 0/0 that dies at the next SBA check — check
  the `--save` and the battlefield line on the FIRST frame after resolution,
  never a later one. Change the count with the Horde's spell on the stack (Purify
  the Grave exiling a Zombie card from your own graveyard) and verify it enters
  with the count at RESOLUTION, not at announcement. Put Mentor of the Meek out
  and verify it triggers on a Mikaeus entering with 1 counter and does NOT on one
  entering with 3 (603.6b). Add Heartless Summoning for the 7c-vs-7d layer check
  — and read the X-funding cap while it is out, which is where #298 lives. Feed
  the Horde one damage source at a time and verify ALL of it is prevented for
  exactly one counter. Untried: two sources damaging the Horde simultaneously
  (CR 614.5 — one counter or two?), and Ludevic's Test Subject's transform.
  Needs one-off decks
- L35 [tried 2026-09-06; every CR claim held, see #65 and #243] lethal damage,
  deathtouch and trample assignment (CR 510.1c-d, 702.2b-c, 702.19b, 704.5g,
  514.2): Kessig Wolf Run's {X}{R}{G} is the only route to a deathtouch trampler,
  so pump a Typhoid Rats and double- or triple-block it — lethal is 1 per blocker
  (702.2c) and every other point tramples (702.19b). Also: damage already marked
  lowers what an ordinary trampler must assign (510.1c); Dead Weight's -2/-2
  kills a creature carrying damage that never touched its toughness (704.5g);
  marked damage clears at cleanup (514.2); a 0-power deathtoucher destroys
  nothing (702.2b). The defects here are the assignment choice never being
  offered (#65 — with trample that silent choice now moves life totals, not just
  which blocker dies) and the granted keyword never being shown (#243). Write
  one-off decks; no coverage pairing has both halves
- L36 [tried 2026-09-06 → #297; every CR claim held] protection and the DEBT
  rules (CR 702.16a-e, 509.1b, 701.17): Spare from Evil grants "protection from
  non-Human creatures until end of turn"; Grave Bramble and Elite Inquisitor
  carry static protection from subtypes, so there are three ways in, not one.
  Verify all four letters — Damage prevented with the battlefield line showing no
  damage marker (702.16d), Enchant/Equip, Block ABSENT from the legal set rather
  than offered-then-rejected (509.1b), Target gone from the menu — and then what
  protection does NOT stop: a spell or Equipment source, because it is protection
  from non-Human *creatures* and a Brimstone Volley must still kill it; sacrifice;
  -X/-X; a 0-toughness SBA. Check the two asymmetries (it may still block a
  non-Human, and a Human may still block it) and that it is gone next turn. Put
  Mask of Avacyn on the same creature for the contrast that makes the keywords
  distinguishable: hexproof removes it from an opponent's target list where this
  protection never does. Unreached: the -X/-X and destroy-all halves — build a
  deck that actually draws Dead Weight, and note a sorcery sweeper can never be
  cast while an until-EOT grant from the other seat is up
- L37 [tried 2026-09-07 → #330, #332] abilities that function outside the
  battlefield (CR 112.6, 113.6): flashback, Skaab Ruinator's graveyard cast,
  Burning Vengeance, Runic Repetition and Back from the Brink audited as one
  family. Write one-off self-mill decks — Armored Skaab and Dream Twist fill a
  graveyard in five turns where a 60 never will. Verify a flashback card is
  offered only to the player whose graveyard it is in (CR 702.34a — put a Dream
  Twist in EACH graveyard and count the menu rows), that a flashback SORCERY and
  the Skaab Ruinator graveyard cast are absent from an upkeep/draw/end-step menu
  rather than offered-and-rejected, and that all three exits exile: resolution, a
  counter (Frightful Delusion while the caster is tapped out) and a fizzle
  (flashback Devil's Play for X=0 at your own 3/1 Kessig Wolf, then Geistflame it
  in response — CR 608.2b). All of that held, as did Skaab Ruinator's cost paying
  at announcement from your OWN graveyard only (CR 601.2f/h) and leaving the menu
  below three creature cards, and Runic Repetition's "exiled + you own + has
  flashback" filter — including the positive case of a card exiled by Purify the
  Grave rather than by flashback. What broke: Burning Vengeance tests
  `cast_with_flashback` instead of "cast from your graveyard", so Skaab
  Ruinator's own permission (and a Rooftop Storm cast) never trigger it (#330),
  and every exile-zone target renders as `obj#NN` because `perm_name` skips exile
  (#332). Unreached: Back from the Brink, Past in Flames, Snapcaster-granted
  flashback under Burning Vengeance, Memory's Journey, Mirror-Mad Phantasm,
  Corpse Lunge, Harvest Pyre, Make a Wish, Creeping Renaissance — budget a deck
  with a real six-land mana base, this one stalled on three
- L38 [tried 2026-09-07 → #323, #324] the replacement effect that turns one event
  into a completely different one (CR 614.1, 616.1): **Undead Alchemist** replaces
  a Zombie's combat damage to a player with a mill, so the damage never happens
  and everything downstream of "was damage dealt?" must agree. Put Curse of
  Stalked Prey on the defender and Curiosity on an attacking Zombie and verify
  neither fires, while a non-Zombie (Moon Heron) in the SAME damage step still
  moves life and still triggers the Curse. Attack with four Zombies at once —
  four separate mills, one per source, not one of the sum (the 2011 ruling) — and
  check a blocked Zombie mills nothing, a token Zombie mills normally, and a
  trampling Skaab Goliath spills only the excess past a Grave Bramble whose
  protection prevents the assigned lethal (702.19b + 702.16d). Two Alchemists
  exile the card once and make two tokens. Verify the second ability is
  opponent-only and library-only (Dream Twist at yourself and Armored Skaab's
  self-mill must NOT fire it; a discarded creature must not either) and that
  milling to zero loses on the next DRAW (104.3c, 704.5b), not when the library
  empties. All of that held. Where it broke was CR 616.1: the affected player
  never chose the order when two effects modified one damage event — `damage.rs`
  ran prevention → multiplier → replacement with no prompt, so Inquisitor's
  Flail on a Zombie milled 2×power (#323), and `replacement.rs::apply` still
  documented that pool as unreachable. Fixed: the defender is now asked, as a
  numbered choice naming what each effect would do, whenever the order changes
  the outcome — re-verify Flail + Alchemist (mill 2 if the Alchemist is chosen
  first, 4 if the Flail is), Ghostly Possession + Alchemist (the defender may
  take the mill), and that Flail + Possession on one creature asks nothing.
  Unreached: lifelink on a Zombie is
  impossible in ISD (Butcher's Cleaver grants it only to Humans), so "no damage
  event means no lifelink" stays unverified; Ghoulcaller's Bell's "each player
  mills a card" as a one-event two-player probe went undrawn in two games
- L39 [tried 2026-09-07 → #319, #333, #334] the card NAME as a game object: the
  legend rule, "same name" effects, copies and Nevermore. Write three one-off
  decks — 4-ofs of Olivia Voldaren / Grimgrin, Corpse-Born / Liliana of the Veil
  with Evil Twin and Cackling Counterpart; a WBG Sever the Bloodline deck with
  Moan of the Unhallowed and the Gatstaf/Estwald werewolf DFCs; a WU Nevermore
  deck with Think Twice (for the flashback) and Urgent Exorcism (to kill the
  Nevermore). CR 704.5j is per player AND per name: two players may each keep
  their own copy of the same legend (an Evil Twin copying the opponent's Grimgrin
  is the cheapest way in), only the duplicate's controller is offered the
  keep-choice, and the loser goes to its OWNER's graveyard — Olivia's {3}{B}{B}
  stealing the other Olivia is the one line in the pool that tests 404.3, and it
  needs the THIEF to hold the two black sources. A copy fires the rule too, and a
  losing token ceases to exist (111.7) while a losing Evil Twin reverts to "Evil
  Twin" in the graveyard (400.7). Sever the Bloodline's Oracle is "all other
  creatures with the same name" with NO controller clause (check
  `data/oracle_cache.json` before believing otherwise): it takes every same-named
  creature on the battlefield, tokens of both players included, and gathers by
  the face that is up, so a Sever on Villagers of Estwald leaves a transformed
  Howlpack of Estwald standing (712.8a). Nevermore's chooser is a filtered card
  list: verify the banned spell and its FLASHBACK are absent from both seats'
  menus while plainly affordable, that a card in neither deck is nameable and
  harmless, and that the ban lifts in the same priority window the enchantment
  dies. All of that held. What did not: `become_copy_of` writes an empty
  instance-effect list, so anything entering as a copy loses the copied card's
  static abilities (#319 — a copied Grimgrin untaps and attacks every turn), and
  no pane, log line or board text ever prints the Legendary supertype (#333).
  Unreached: Essence of the Wild (not drafted into any deck), Geist of Saint
  Traft's and Garruk Relentless's own legend-rule instances, and whether Garruk
  Relentless and Garruk, the Veil-Cursed correctly coexist under one controller
- L40 [tried 2026-09-08 → #358, #359; every CR claim held] the untap step (CR 502)
  and "doesn't untap during your untap step": Claustrophobia, Galvanic Juggernaut,
  Grimgrin Corpse-Born, Avacynian Priest, Spidery Grasp and the vigilance creatures,
  in one-off WU/GW-vs-UB probe decks. Verified that no priority is offered in untap
  (502.4 — the CLI never prompts, the log goes Untap→Upkeep) across ~50 untap steps,
  that only the ACTIVE player's permanents untap (502.1), and that the restriction is
  read AT the untap step, not latched when the permanent was tapped — the sharpest
  route is an Avacynian Priest tapping an UNTAPPED Grimgrin at the end of its
  controller's turn, which then stays tapped on their next untap step. Also verified:
  removing Claustrophobia AFTER the untap step leaves the creature tapped for a whole
  turn cycle while removing it BEFORE lets the creature untap; a Juggernaut untaps
  mid-combat from its own death trigger and stays a legal attacker (506.4); Grimgrin
  enters tapped (614.1c), is absent from the attacker list while tapped, and its
  sacrifice ability leaves the menu when it is the only creature (601.2h); summoning
  sickness clears at the untap step even for permanents that did not untap (302.6);
  vigilance means there is nothing to undo. Both defects are in the log. UNREACHABLE
  in this pool: a "doesn't untap" permanent under a NEW controller at an untap step —
  Traitorous Blood expires at cleanup and Olivia needs 5 power — and any trigger that
  fires during the untap step, so the "waits for upkeep" half of 503.1 is untested
- L41 [tried 2026-09-08 → #355, #356; every CR 701.12 claim held] the fight event:
  only TWO cards reach it — Prey Upon and Nightfall Predator (Daybreak Ranger's back
  face). Garruk Relentless's 0 ability is NOT a fight and is correctly implemented as
  two damage instructions; don't re-check it here. Verified: simultaneous damage
  (701.12a/704.5g), illegal target → neither creature deals or is dealt damage on both
  the spell and the ability path (701.12b, via Ranger's Guile hexproof and via killing
  the target in response), self-fight at twice power (701.12c), deathtouch (702.2),
  indestructible with damage still marked (702.12b), fight during combat counting
  toward lethal at the same SBA check, Prey Upon's two target restrictions enforced
  separately and hexproof creatures absent from its menu (702.11b), and fight damage
  surviving BOTH Moonmist and Ghostly Possession because it is not combat damage
  (701.12d). Unreached in two games: Creepy Doll's "deals combat damage to a creature"
  trigger and a 0-power fighter (Tree of Redemption) — both look right in source; stack
  them higher and give the target seat more ramp, it stalled on three lands both games
- L42 [tried 2026-09-08 → #326, #356 comments; both 603.4 checks held] the intervening
  "if" clause: an intervening-"if" trigger is checked twice — it does not trigger unless
  the condition is true at the event, and it is removed from the stack if the condition
  is false on resolution. The scoping fact this night established, which shapes any
  future attempt: NO implemented card has an intervening-"if" condition that can change
  between trigger and resolution. Every one is "last turn's spell count" (the 12
  werewolf DFCs, both faces) or "a creature died this turn" (Woodland Sleuth, Morkrut
  Banshee, Hollowhenge Scavenger, Reaper from the Abyss), both frozen once the trigger
  fires, so the classic "kill the Zombie in response" demonstration is unbuildable here
  — and Endless Ranks of the Dead has had that clause errata'd off its Oracle text. What
  held: no trigger on the stack on six upkeeps where the condition was false; Morkrut
  Banshee entering with nothing dead produced no trigger and no phantom target prompt;
  Reaper sat through an end step with two legal targets and nothing dead; a spell cast
  in response never stopped a werewolf trigger; and Moonmist cast in response to Gatstaf
  Shepherd's OWN trigger flipped it forward and let the trigger flip it back (603.4 +
  712.8, via `resolving_trigger_from_back_face`). Also confirmed each-upkeep scope and
  603.3b ordering
- L43 [tried 2026-09-11 → no bug; every check held] the intervening "if" that is still
  true when the target has gone. Both cards were reached and both are CORRECT. Reaper
  from the Abyss: morbid on (Altar's Reap sacrificing your own creature, or any combat
  death), the end-step trigger targeting your own Typhoid Rats, that Rats sacrificed to
  a second Altar's Reap in response → `Reaper from the Abyss (#21)'s end step trigger
  (if morbid, destroy target non-Demon creature) fizzled (all targets illegal)`, and the
  opponent's creature untouched. Morkrut Banshee's ETB: same shape, with the Banshee
  chosen as its own target and then killed by Victim of Night in response → `fizzled
  (all targets illegal)`, and no creature took -4/-4. Neither log line mentions morbid,
  which is what the probe was for. Three facts this night established that shape any
  repeat:
  * the THIRD state is reachable and is distinguishable from both others. Morbid TRUE
    with no non-Demon creature anywhere (two Reapers out, sacrifice one, opponent's board
    empty) logs `Trigger removed: no legal targets (Reaper from the Abyss (#21)'s end
    step trigger ...)`; morbid FALSE logs NOTHING AT ALL. Four end steps with the Reaper
    out and morbid false produced zero trigger lines, so the 603.4 gate and the 608.2b
    fizzle cannot be confused in the log — which is exactly the regression the comment in
    `reaper_from_the_abyss.rs` was written against.
  * the pairing in the old budget does not work. Victim of Night is "non-Vampire,
    non-Werewolf, non-Zombie", so it cannot remove any target on a Zombie deck's board —
    the response has to be Altar's Reap sacrificing your OWN creature, or the target has
    to be something of yours. Give the Reaper seat its own non-Demon bodies.
  * the target prompt only appears when there is a real choice. With one legal target
    the engine locks it in silently, which reads like an auto-choice if you are sending
    keystrokes blind — check the stack pane (`s`) before concluding you were not asked
- L44 [tried 2026-09-11 → #469; the audit is DONE, don't redo it] the FIRST 603.4 check,
  card by card. The premise it shipped with is stale: Homicidal Brute has had a
  `should_trigger` gate for a while, and all twelve werewolf DFCs route through one
  shared `helpers::werewolf_should_trigger`, so "find the ungated card" finds nothing.
  What the night verified at runtime, in three games under `--check-invariants`:
  * Homicidal Brute in SIX configurations, all correct. Attacked this turn → nothing on
    the stack, nothing in the log, no transform back (turns 7, 11, 15, 16 of one game).
    Did not attack → the trigger fires, taps and flips it back, with a real priority
    window. Attacked on a PREVIOUS turn → the trigger still fires, because
    `attacked_this_turn` is `attacked_on_turn == Some(turn_number)` (`state.rs:3139`)
    rather than a flag someone has to clear. "Your end step" is the CONTROLLER's
    (CR 603.2) — a Brute held through the opponent's end step produces nothing. A Brute
    stolen by Traitorous Blood after attacking triggers for the THIEF on the thief's end
    step, and a Brute stolen and then attacked with produces nothing — the stamp is on
    the object, not the original controller. The front face declares no trigger at all.
  * all three morbid ETBs with the condition false — Woodland Sleuth, Morkrut Banshee,
    Hollowhenge Scavenger — log only their resolve line, show an empty stack, and (the
    Banshee) offer NO target prompt, which is the 603.4-vs-603.3c distinction the card's
    doc comment argues for. The other morbid cards need no gate: Somberwald Spider and
    Festerhide Boar are replacement effects, Skirsdag High Priest is an activation
    restriction, Caravan Vigil and Brimstone Volley are spell effects.
  * both werewolf faces in both directions, including the one-spell limbo where neither
    face wants the turn (sum == 0 false, any >= 2 false → nothing triggers), and 603.3b
    ordering when two back-face werewolves trigger together.
  The one defect is #469, and it is not a missing gate but a gate testing the wrong
  thing. Three configurations are UNREACHABLE in this pool and should not be chased
  again: a Brute attacked and stolen in the SAME turn (Traitorous Blood is a sorcery),
  attacking as the Scholar and then transforming (the `{T}` ability cannot be activated
  after the Scholar taps to attack — only transform-then-attack is buildable, which is
  what the unit test covers), and a Brute removed from combat after attacking (nothing
  in ISD removes an attacker without moving it to another zone; tapping an attacker does
  not remove it from combat, CR 506.4)
- L45 [tried 2026-09-11 → #467, #468] the destroy pipeline as a REPORTING contract, not
  a rules one. `mtg-engine/src/destruction.rs` returns a `DestroyResult` (Died /
  Regenerated / Indestructible / NotAPermanent) and `try_destroy_by` exists so that the
  line naming the source is the true one. Audit every caller that writes its own
  "X destroyed Y": Witchbane Orb and Paraselene branch on the result and are correct,
  Divine Reckoning writes no line at all, the five converted cards (ghost_quarter,
  creepy_doll, into_the_maw_of_hell, evil_twin, maw_of_the_mire) are correct — but the
  shared `PendingEffect::Destroy | DestroyCreature` arm in `engine/effects.rs` throws the
  result away and logs "destroyed" unconditionally (#467). Two cards route through it:
  Slayer of the Wicked and Reaper from the Abyss. Both arms reproduce. The two cheapest
  boards, and worth reusing for anything in this area: a Walking Corpse wearing Skeletal
  Grimace ({B}: Regenerate) against Slayer of the Wicked reaches the regeneration arm on
  4 lands; Manor Gargoyle ({5}, colourless so it fits any deck, indestructible as long as
  it has defender) against Reaper from the Abyss reaches the indestructible arm, where
  the false line is the log's ONLY statement about the event. The general move: find
  every place a card announces an outcome it did not check, and the general shape of the
  answer is a helper that takes the result. While you are there, a live regeneration
  shield is rendered nowhere (#468) — `regeneration_shields` does not appear anywhere in
  `mtg-player/`, so neither the battlefield line nor the `i` detail screen shows it
- L46 [proposed 2026-09-11, from L44 and #469] the intervening "if" versus the
  RESOLUTION impossibility. `helpers::werewolf_should_trigger` refuses the upkeep trigger
  for a token copy on the grounds that a token cannot transform — which is CR 701.28c,
  a fact about resolution, not part of the printed clause CR 603.4 tests. The engine
  already disagrees with itself about it: `apply_transform` refuses tokens AND
  single-faced clones, but the gate only knows about tokens, so an Evil Twin copying a
  werewolf triggers and does nothing (correct) while a Cackling Counterpart token of the
  same werewolf never triggers (#469). Sweep every other `should_trigger` in
  `cards/isd/` for the same conflation: a gate may test the printed condition and
  nothing else. Then go the other way and look for the mirror — a resolution handler
  that silently does nothing where the ability should not have triggered at all, which
  is the shape `reaper_from_the_abyss.rs`'s comment records having had
