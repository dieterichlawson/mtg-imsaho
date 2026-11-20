# Cards verified with the engine in the loop

Started 2026-11-11. The question this plan answers: does each card really
work, in the engine as it runs, on the boards and in the turns a game reaches?
Not "does the card's code ask for the right engine calls", which is what the
Kani card proofs check.

## Where the card checks stood

Two layers check cards against their Oracle text.

- **The Kani card proofs** (`mtg-engine/src/proofs/cards/`, 990 harnesses).
  Each runs one card's code on a symbolic board, with about fifty engine
  actions replaced by recorders (`kani::stub`), and asserts the list of
  calls the card made. Of the 51 recorders, 26 record and do nothing else,
  and several return a fixed success (`record_move`, `record_attach`,
  `record_counter`, `record_transform`, `record_move_under`). So a proof
  says what the card asked for, against a specification that is close to
  the card's own code. It says nothing about the game state that results,
  and code after a call reads a state the call did not change, or never
  sees the call fail. The engine actions are proved separately
  (`proofs/actions/`), but nothing ties a recorder to the action it stands
  in for. Of the 40 defects found before this plan, 3 were found by card
  proofs.
- **The independent reference** (`mtg-engine/tests/card_semantics.rs`). A
  grammar reads every card's Oracle text, all 268 of the pool, and a
  reference semantics written from the Comprehensive Rules says what each
  reading does to a board. The real engine casts, activates and triggers
  each card, end to end, at every target set and every answer to every
  question, and the games it ends in are held to the games the text
  allows. This is the engine in the loop. Its weakness was the boards:
  each card was cast on about seven fixed boards, each written for what it
  tests and kept to cards "that change nothing except through what they
  are".

## The Kani pilot: the real engine under a card

Before choosing, the obvious fix was tried: run a card proof with no
recorders, so that the card's code calls the real engine, and assert the
state the engine leaves. Doom Blade, the simplest destroy spell, on the
proof board of `proofs::cards` (`kani-bounded.py`, 12 GB, one at a time):

| Harness | What is open | Result |
|---|---|---|
| `on_resolve`, real `try_destroy` and `move_object` | one object of any colours, types, keywords, power, toughness, zone and controller; tapped, damage, a shield | out of memory, 115 s |
| the engine resolving it off the stack (`resolve_top_of_stack`) | the same | out of memory in symbolic execution, 554 s |
| that, then state-based actions | the same | out of memory in symbolic execution, 568 s |
| `on_resolve`, lists fixed | controller, power, toughness, tapped, damage, shield, indestructible | out of memory, 81 s |
| resolution off the stack, lists fixed | the same | out of memory in symbolic execution, 561 s |
| `on_resolve`, everything fixed | whether it is tapped | proves, 60 s (277,557 steps) |

The engine's own proofs of `try_destroy` pass only because they stub
`move_object` and `has_keyword`. Composed, the real zone change under a card
is out of reach for CBMC, with the scalars alone left open. So Kani cannot
hold the engine under a card except on boards that are essentially concrete,
which is a test, not a proof. The pilot's harnesses are not kept.

(That reading was wrong about a card's own hook: part B2 measures it one
freedom at a time. The "lists fixed" rows above did not fix the list that
mattered.)

That settled the split as first planned: the engine in the loop as the
reference's job (part A), and the Kani card proofs kept compositional and
made sound about the engine by stand-ins that model each action (the
first part B). The second half did not stand. A card proof that does not
run the engine is not a proof that the card works, whatever its stand-ins
model, and on 2026-11-18 the recorder harnesses were removed and part B
rewritten: a card's proof runs the engine itself.

## Part A: the reference on generated boards

### A1. Generated boards (done)

`generated_board(seed, ..)` draws a board from a seed (SplitMix64, the same
board for the same seed everywhere): up to five permanents on each side
from the pool's permanents the reference reads whole, or creature tokens of
any power, colour, subtype and keyword; each tapped or not, damaged short
of lethal, with +1/+1 or -1/-1 counters, a regeneration shield, or the
other player's card; up to four cards of the pool in each graveyard; life
from 6 to 20. A board that the reference says does not stand is cut back
until it does, and `every_generated_board_is_what_its_cards_print` holds
the engine to the reference on each board before anything is cast, and to
nothing changing under state-based actions.

`every_readable_card_does_what_its_text_says_on_generated_boards` casts
every card the grammar reads on each board, through the same per-board
check as the fixed boards (`check_cast_on`). `CARD_SEMANTICS_BOARDS`
(default 4) and `CARD_SEMANTICS_FIRST_BOARD` choose the boards.

The first four boards disagreed on 128 of 3,443 casts. Every one was
worked to its cause:

- **One engine defect.** A token with no name of its own was named after
  its subtypes alone ("Spirit"); CR 111.4 names it after its subtypes plus
  the word "Token" ("Spirit Token"; the rule's example is Dwarven
  Reinforcements' "Dwarf Berserker Token"). Issues #331 and #334 had moved
  the engine to the subtypes alone, quoting an older edition of the rule
  from memory. It shows when a card copies a token: Evil Twin entering as a
  copy of a Spirit token became a card named "Spirit". A token of no
  subtype and no name was nameless, where the rule makes it "Token". Fixed
  in the engine and in its object invariant, which checked the old reading.
  The runner's stub seat answers with a hash of its prompt, so the new
  names sent one of `schema_cache.rs`'s seeded games down another path,
  asking X funding twice where the test wants three; it now plays a second
  seed of that deck.
- **A harness defect.** A game that came to rest with the caster having
  nothing left to do was played on by the loop's own passes into the next
  turn, its untap and draw done, before the harness saw it: Murder of
  Crows' "draw, then discard" that discarded the caster's last land ended
  with the opponent's draw. The harness now sees every pass
  (`observe_every_submit`) and stops where the game rests.
- **Gaps of the reference's own**, each a rule it had not needed on boards
  written for it, and each now read as the rules say:
  - a characteristic-defining power is what "power 4 or greater" reads
    (Smite the Monstrous and a Geist-Honored Monk with five creatures);
  - protection keeps a creature from being targeted by what it has
    protection from (CR 702.16b, Crossway Vampire's trigger and Elite
    Inquisitor) and prevents a fight's damage from it (702.16e);
  - hexproof keeps a creature from being "any target" of an opponent, not
    only "target creature" (702.11b);
  - Abattoir Ghoul's "dealt damage by" counts a fight's damage;
  - a token has a name (CR 111.4), which "the same name" reads (Sever the
    Bloodline aimed at a token) and a copy of it takes (707.2);
  - a token exiled by a linked ability ceases to exist and does not return
    (111.7, 111.8; Fiend Hunter);
  - a permanent that enters other than by being cast, returned from a
    graveyard or from exile or made as a token copy, enters as its text
    reads (tapped, with its counters, its statics and combat text), and its
    enters trigger triggers, the active player's first and each player's in
    any order (603.3b), its targets chosen as it goes on the stack:
    Unburial Rites returning Grimgrin, Moldgraf Monstrosity returning a
    Bloodcrazed Neonate, Fiend Hunter returning a Geist-Honored Monk,
    Cackling Counterpart copying one, Evil Twin entering as a copy of a
    Crossway Vampire;
  - "that creature's toughness" is as it last existed (608.2h; Tribute to
    Hunger and a Monk whose toughness counts itself);
  - "unless its controller pays {1}" may be paid from any untapped land or
    artifact whose mana ability costs only {T} (Gavony Township), each way
    it can be;
  - a card returned from a graveyard counts itself where its counters count
    the cards there: Unbreathing Horde's ruling (2011-09-22), which the
    engine already followed.
- **Two assumptions of the harness's**, true of the fixed boards only: that
  lethal damage kills each creature a death watcher is shown dying (not an
  indestructible or shielded one), and the game coming to rest, above.

Each found by a board none of the fixed ones was. After them, every card
agrees with its text on the first four boards.

### A1, continued: deaths on the boards, and 1,024 of them

The first widening of A2 came next, because sixty-four boards needed
it: a permanent with a dies trigger or a watcher of deaths may now be on
a board. At most one, none beside a card under test that is one, and a
board holding one holds at most four creatures, so that the orders its
triggers may go on the stack in stay countable (a board wipe over five
death watchers is 5! orders of each player's). The reference reads the
deaths as the rules have them: each creature that dies records its last
known information, its own dies trigger and every watcher's, on the
battlefield or dying beside it, triggers (CR 603.6c, 603.10a), and they
go on the stack in APNAP order, each distinct order of each player's own
(603.3b).

Sixty-four boards then disagreed on 4 of 36,549 casts, boards 64 to 320
on 4 more, and boards 320 to 1,024 on 15. Each was worked to its cause.
Six engine defects:

- **Defect 42.** Corpse Lunge read the exiled card's power in exile, as
  it resolved; the ruling on the same words (Corpse Explosion,
  2022-04-29) reads it as it last existed in the graveyard. Boneyard
  Wurm left itself out of its own count. Issue #576 had chosen the
  reading in exile on purpose; it is reversed.
- **Defect 43.** A spell's additional costs were paid with the card
  still in its owner's hand, where CR 601.2a moves it to the stack first:
  Sturmgeist exiled to Corpse Lunge counted the Lunge in its hand.
- **Defect 44.** Creatures returned together (Grimoire of the Dead,
  Moldgraf Monstrosity) were moved one at a time, each seeing the ones
  before it: an Essence of the Wild among them made the rest copies of
  itself, a Dearly Departed among them gave the Humans after it nothing,
  and an Unbreathing Horde counted what had just arrived. CR 614.12 works
  each entry out against the effects that existed before the event; the
  engine now plans them all before any moves.
- **Defect 45.** An Evil Twin entering under its controller's Essence of
  the Wild was what it chose, where the Essence's ruling makes it an
  Essence "no matter what order these effects are applied". Found reading
  the code for defect 44; the reference had it wrong the same way.
- **Defect 46.** Dearly Departed read an entering permanent's own
  subtypes, so an Evil Twin entering as a copy of a Human was no Human to
  it (CR 614.12, 707.2).
- **Defect 47.** A spell's targets were chosen with the card still in its
  owner's hand, where CR 601.2a has moved it to the stack first: Smite the
  Monstrous was offered a Sturmgeist that counted the Smite among the
  cards in its controller's hand.

And the reference's own gaps, each closed: counters annihilating (CR
704.5q) and surviving a transform; a copy entering tapped and with the
counters its copied text gives it (an Evil Twin copying a Festerhide
Boar after a death); a Dearly Departed's counter on a copy of a token
and on a token; creatures' mana abilities, and abilities whose cost
mills, paying "unless its controller pays"; enter watchers for what
arrives; a creature entering with no toughness dying, with what watches
it; a draw from a short library; a negative power gaining no life
(107.1b); the triggers paying a cost sets off resolving before the
spell (601.2i), after the state-based actions the cast is followed by
(704.3: a Boneyard Wurm that loses the last creature card it counted
dies before the spell resolves), and a spell whose every target has left
not resolving (608.2b); a copy leaving the battlefield as the card it is
(400.7: an Evil Twin copying a Boneyard Wurm dies an Evil Twin); and a
watcher that died seeing only the deaths of its own event and those
before it (603.10a: a Falkenrath Noble dying beside a Gallows Warden
does not see the Spirit that dies of the Warden's leaving). Two harness
gaps: a random trigger a cost sets off (Moldgraf Monstrosity sacrificed
to Altar's Reap) was played from one seed, not 32, and a resolution that
settled one way was compared as it stood before its copy choices were
made.

After them every card agreed with its text on 1,024 boards, about
580,000 casts of the 268 readable cards, in two runs of the same build.
Two earlier runs of that build each reported one disagreement that no
rerun reproduced (Moldgraf Monstrosity on "board 693", 20 runs of it
alone agreeing; and a reference panic). That was explained on
2026-11-18, and was the harness's: the cards the grammar reads were
listed in the registry's order, a hash map's, which is each process's
own, and a board is drawn from places in that list. So a seed drew
another board in each run: the two runs were of two sets of 1,024
boards, both agreeing, and the two disagreements were on boards no
rerun made, which is why they are lost. The list is now in the order of
its names (`the_cards_read_come_in_one_order` holds it there), a seed
draws one board everywhere, and the 1,024 are to be run again as the
boards they now are.

Known and not yet fixed, each with a test or a note where it shows:

- **"Unless pays" is paid by the planner.** A cost the engine asks for as
  a spell resolves (Frightful Delusion's) is paid from
  the sources its autotap planner picks, not as the payer chooses (CR
  601.2g-h, 605.3a). The reference compares those endings by
  `disagreement_paying`: each the engine reaches must be one the text
  allows, and it must reach a paying one and a declining one.
- **Two Essences.** Of two Essences of the Wild, the engine applies the
  one with the higher id last; CR 616.1 lets the entering creature's
  controller choose. Both copies are 6/6 Avatars unless one Essence is
  itself a copy with something added (an Evil Twin's ability).
- **A deferred copy's own count.** An Evil Twin returned beside others
  and copying a card whose own entry counts the board (an Unbreathing
  Horde) counts the board as it is when the Twin is answered, the others
  already there.

### A2. Wider boards (next)

The vocabulary leaves out, each for a reason the reference does not yet
carry to a board it did not write:

1. what watches deaths is in (above), one to a board; still out are
   upkeep and end-step triggers, creatures that attack each combat, death
   triggers that search or shuffle a library, and more than one death
   watcher on a board;
2. Auras and Equipment attached to what the board holds;
3. double-faced cards on either face;
4. statics that change costs, tokens, entries, mills, targeting or
   activation (Parallel Lives, Essence of the Wild, Witchbane Orb, Stony
   Silence and the rest), each of which has a test of its own on a fixed
   board;
5. planeswalkers, and life totals a spell can take to nothing.

Each widening is either found to agree, or finds an engine defect or a gap
of the reference, and the gap is closed before the next.

### A3. Every property on generated boards

The main property is the first. The others cast on fixed boards too:
activated abilities, loyalty abilities, werewolves, upkeeps and end steps,
flashback, watchers, combat. Each moves to a generated board in turn.

### A4. In the middle of things

What a card does when the board changes under it: the opponent responding
with each instant and ability of the pool before it resolves, its target
made illegal, its source gone (defects 36 and 40 are of this kind). The
activated abilities are already played again with their source exiled in
response; spells and triggers are not.

### A5. Real games

The fuzzer reaches states nobody writes or generates. A shadow check in the
fuzz runner: at each resolution of a card the grammar reads, the reference
reads the board as the engine has it, says what the resolution may end in,
and the engine's ending is held to it. This needs the reference to read any
board, which A2 builds toward.

### Scale

`cargo test` runs four boards per card (about 20 s in a debug build). A
release build casts every card on 64 boards in about 20 s and on 1,024
in about six minutes (`CARD_SEMANTICS_BOARDS=1024 cargo test --release
-p mtg-engine --test card_semantics -- generated_boards`), sharded with
`CARD_SEMANTICS_FIRST_BOARD`.

## Part B: card proofs that run the engine

Decided 2026-11-18: no card proof stands in for the engine. What that
leaves, in the order it is being done:

### B0. The recorder harnesses, removed (done)

715 of the 990 Kani card harnesses asserted a record of calls: 11 went
with master's simultaneous entries (#1064-#1068), whose function their
stand-in had replaced, and 704 on 2026-11-18. What is kept in
`proofs/cards/`, 285 harnesses, is what a card declares (abilities,
costs, targets, triggers, statics, replacements) and what its own code
writes to the state (a question, an until-end-of-turn effect), each read
by an engine function with a proof of its own. No stand-in is between
those assertions and the code they are of.

### B1. Every small board (under way)

The generated boards are a sample. A proof by exhaustion is the same
check over every board of a stated size, and that is what a card's proof
now is: the real engine under the card, every case of a family counted
out whole, and the reference as the specification.

**The family.** A board is what every board holds (the hands and
libraries of the fixed board, 20 life each, the first player's main
phase) and a multiset of at most N pieces. A piece is a permanent of the
vocabulary on either side in a state of wear (as it entered, tapped, the
other player's card, 1 damage, a +1/+1 counter, a -1/-1 counter, a
regeneration shield; the last four a creature's), a creature token on
either side likewise (the nine kinds the set makes or nearly, and two of
no colour or subtype), or a card of the pool in either graveyard: 2,170
pieces, 808 of them as they entered. Boards of one piece wear every
state; larger boards are as they entered unless asked otherwise
(`CARD_SEMANTICS_SMALL_WORN`). Not boards of the family, and counted as
such by each run: one whose permanent cannot be worn as asked or is left
with no toughness (dead, it is another board), and one with two
permanents that trigger on deaths.

**What is played on each board** (`on_every_small_board`), each through
the same check as on the hand-made boards, now a function of the board:

| | what | one piece (2,032 boards) |
|---|---|---|
| casts | every non-land card the grammar reads, cast at every target set and every answer (`check_cast_on`) | 568,483, 37 s |
| activated abilities | every one read, of either face, its permanent placed under the player to act: offered at exactly its targets and sacrifices, paid, resolved every way (`check_activated_on`) | 111,760, 24 s |
| flashback | every flashback read, cast from the graveyard (`check_flashback_on`) | 61,226, 15 s |
| upkeeps | every upkeep trigger and "doesn't untap" read, in each player's turn (`check_turn_begins_on`) | 55,396, 5 s |
| end steps | every end step trigger read, in each player's turn (`check_turn_ends_on`) | 16,256, 2 s |
| loyalty abilities | every one read, of either face, at its printed loyalty and at its cost (`check_loyalty_on`) | 22,000, 5 s |
| werewolves | every werewolf read, transforming after a quiet turn and back after a turn of two spells (`check_werewolf_on`) | 24,384, 2 s |
| enter watchers | every "whenever ... enters" read, as each player casts a creature it may or may not name (`check_enter_watcher_on`) | 12,192, 2 s |
| cast watchers | every "whenever you cast ..." read, as its controller casts an instant from their hand and by flashback and a creature, and as the other player casts an instant (`check_cast_watcher_on`) | 24,384, 3 s |
| combat | in the first player's turn: the attackers offered and required, the blocks offered for each set of attackers, and each way of blocking fought to its end (`check_combat_on`) | 533 combats, under a second |

Every one of those agrees with its card's text. They run in
`scripts/prove.sh exhaustive`, and so on every push and pull request.
`CARD_SEMANTICS_SMALL=2` is every board of two pieces: 327,645 boards,
about 91 million casts and 18 million activations, some six hours for the
five on four cores, run in shares (`CARD_SEMANTICS_SHARD=K/N`); a failure
names its board, and `CARD_SEMANTICS_SMALL_BOARD=I,J` replays it alone.

**What it has found so far.** One engine defect, and nineteen gaps of
the reference, each on a board nobody wrote.

The defect (48 in `formal-verification.md`): two triggers of one ability
of one source went on the stack front first, their controller not asked,
also where each names a different creature to damage. An Ashmouth Hound
blocked by an Unruly Mob and a Voiceless Spirit lives if the Mob is dealt
its 1 damage first and dies if the Spirit is (the Mob, grown by the
Spirit's death, survives its own and trades with the Hound), and the
engine played only the second game; CR 603.3b gives the order to the
Hound's controller. Combat on three pieces found it, in the sixth share
of sixty, as 2 of 746,372 combats. The reference had the step wrong too,
another way, which is the last gap below.

The gaps, in each of which the engine was right:

- the legend rule (CR 704.5j): two Grimgrin, Corpse-Born returned
  together by Moldgraf Monstrosity (two pieces);
- a token made as a resolution goes and dead before it ends (Stitcher's
  Apprentice's Homunculus, sacrificed to the ability that made it) is a
  death a Lumberknot sees (activated abilities, one piece);
- such a token may be the target of what triggers during that
  resolution (a Pitchburn Devils sacrificed to the same ability);
- "exchange your life total with this creature's toughness" sets the
  toughness and applies the board's effects after (Tree of Redemption
  under Glorious Anthem; the card's ruling of 2018-03-16);
- a card returned at random from a graveyard may be one milled a moment
  before (Woodland Sleuth's trigger after Armored Skaab's; two pieces);
- a creature that arrived this turn does not tap for mana (a Deranged
  Assistant returned beside the Mentor of the Meek whose trigger asks
  for {1}, CR 302.6; two pieces);
- "the exiled card" is each source's own (CR 607.2a): of two Fiend
  Hunters returned together that exile each other, one comes back (two
  pieces);
- a trigger that a resolution sets off goes on the stack above what is
  still there and resolves first (CR 603.3b, 405.5): Fiend Hunter and
  Morkrut Banshee returned together, the Banshee's -4/-4 killing the
  Hunter while its enters ability waits, so that its leaves ability
  returns nothing and the Banshee is then exiled for good (the Hunter's
  ruling of 2018-12-07; two pieces);
- a death in combat sets off what a death anywhere does: the reference's
  combat resolved its own triggers and no dies trigger or watcher of
  deaths, as no hand-made combat had one (Doomed Traveler blocked by a
  Wolf, Falkenrath Noble blocking; combat, two pieces);
- a creature that regenerates is removed from combat (CR 701.19c): a
  first striker's blocker that regenerates in the first-strike step
  deals no damage in the second (Abattoir Ghoul blocked by a shielded
  Falkenrath Noble; combat, two worn pieces);
- "leaves the battlefield" is any way of leaving, not a death alone (CR
  603.6c): of two Fiend Hunters returned together that exile each
  other, the second one's leaving returns the first, which the reference
  had left in exile with it (the one disagreement in 91,095,375 casts on
  the boards of two pieces);
- what was resolving goes on resolving once the triggers its cost set
  off have: Demonmail Hauberk's equip, paid by sacrificing a Falkenrath
  Noble whose death trigger resolves above it, still attaches the
  Hauberk (activated abilities, two pieces);
- the legend rule's choice (CR 704.5j) where the two differ: Grimgrin
  sacrificing a Moldgraf Monstrosity that returns a second Grimgrin, the
  one already there untapped or with a counter; the reference kept the
  first of two alike and gave up on two that were not (activated
  abilities, two pieces);
- what a damage step sets off goes on the stack together (CR 603.3b,
  101.4): the reference resolved the damage's own triggers ("deals
  combat damage to a player", a watcher's or a Curse's counter, an
  Aura's draw, a coin flipped) as the damage was dealt and the deaths'
  after, whoever controlled them. Stromkirk Patrol and Typhoid Rats
  attack, Pitchburn Devils blocks the Rats and dies, and the Devils' 3
  damage, the other player's trigger and so the first to resolve, kills
  the Patrol before its counter arrives (combat, three pieces);
- a trigger whose every target is gone by the time it resolves does
  nothing (CR 608.2b): that Devils' damage at a creature a trigger above
  it has already killed (combat, three pieces);
- "each creature that player controls" is read as the trigger resolves:
  Balefire Dragon's damage reaches the Spirit of a Doomed Traveler that
  died in the same step, its trigger the other player's and resolved
  first (combat, three pieces);
- block triggers resolve one at a time, the state-based actions and what
  a death sets off between them (CR 603.3b, 704.3): the reference
  resolved them all and then looked for the dead, so that two blockers an
  Ashmouth Hound damaged died together and nothing saw the first die
  (combat, three pieces; the board of defect 48);
- what watches a death is whatever was on the battlefield as it
  happened (CR 603.10a), not creatures alone: a Gutter Grime sacrificed
  in one pile with a creature, to Liliana of the Veil's last ability,
  still makes its Ooze, a 0/0 with the Grime gone, and a Glorious Anthem
  in the other pile keeps it alive (loyalty abilities, one board in
  sixty of three pieces);
- a library that has been shuffled is in an order nobody knows (CR
  701.20a), so what a trigger then mills or draws from it is any card
  of it. Memory's Journey, cast by flashback, shuffles the one creature
  card in a graveyard into its owner's library; their Boneyard Wurm
  dies a 0/0 of that; and a Selhoff Occultist's trigger mills the
  library. The reference milled the card its own list had first, and
  the engine, over seeds, each of the six (flashback, one board in 480
  of three pieces: 1 of 6,668,805 casts). The reference now plays every
  way the top of a shuffled library can lie for what a resolution takes
  from it, chosen before anything that depends on the card taken (a
  card discarded after a draw), and counts endings, as it compares
  them, without a shuffled library's order.

And what the checks had taken from their hand-made boards and no longer
do: a board's own triggers, set off by a cost or a resolution, settled
every way in the activated and flashback checks as they were in the
cast check; the opponent's spell held on the stack cast on any creature
of theirs; the source "exiled in response" played only where the
ability is alone on the stack.

One cut, where the reference does not yet read far enough: an ability
or a spell that lasts the turn is played on into the next turn only
where neither its cost nor its resolution set off a trigger: the
reference plays the source's own text for the turn's end and no other
permanent's, and what a trigger brings may have some (a Reaper from the
Abyss returned by a Moldgraf Monstrosity sacrificed).

A second cut is gone. A loyalty ability that searches a library
shuffles it, and what a death trigger of the board's then mills or
draws is any card of it; the reference did not play that, and such an
ability (Garruk, the Veil-Cursed's sacrifice) was kept off any board
with a death trigger. With the last gap above closed it is played
there, the engine over seeds: 43,624 more abilities on the boards of
two pieces, all agreeing, and two such boards in `cargo test`
(`loyalty_abilities_agree_where_a_search_shuffles_under_a_trigger_that_reads_the_library`).

A check that gives up (the reference meeting what it does not play) no
longer ends its share: it is reported as that board's failure, by name.

**Two pieces.** Every board of two pieces as they entered, 325,888 of
the family, in 40 shares:

| Check | Plays | Result |
|---|---|---|
| end steps | 2,607,104 | all agree, against the reference as it now stands (4b60d3c) |
| werewolves | 3,910,656 | the same |
| enter watchers | 1,955,328 | the same |
| cast watchers | 3,910,656 | the same |
| loyalty abilities | 3,584,768 | all agree, a search's shuffle now played under a board's death trigger (3,541,144 before that cut was lifted) |
| upkeeps | 8,921,612 | the same |
| flashback | 10,376,473 | the same, and again whole against the reference as changed for a shuffled library |
| combat | 94,552, and 1,441,016 on the 2,052,641 boards of two worn pieces | all agree |
| activated abilities | 17,923,840 | the same, once three gaps above were closed |
| casts | 91,095,375 | the same, once the gap above was closed |

144 million plays, every one agreeing with the reference as it stood at
4b60d3c; each check was run again whole against it after the last gap
was closed. What combat on three pieces then found (four gaps, and
defect 48 in the engine) changes combat alone, and combat was run again
whole against the reference and the engine as they stand (a24dc8b): the
same 94,552 and 1,441,016 combats, all agreeing, with the exhaustive
tier of the hand-made combats and every board of one piece.

Combat is cheap enough that `prove.sh exhaustive` plays it on every
board of two worn pieces. The boards are walked one at a time now, so
three pieces fit in memory: a slice of them (one share of 4,000) fights
11,112 combats that all agree, and all 88,573,365 boards are some hours
of this machine for combat alone.

**Three pieces, combat (begun 2026-11-19).** The first three shares of
60, 4.4 million boards and 2,237,248 combats: 28 disagreed, on 14 boards,
and every one was a gap of the reference, the engine right. What a
damage step sets off goes on the stack together: a "deals combat damage
to a player" trigger, and the dies triggers and watchers of what the
damage killed. The active player's go on first and resolve last, and a
player orders their own (CR 603.3b, 101.4). The reference resolved the
damage's triggers first and the deaths' after, whoever controlled them.
Two attackers and a blocker is the smallest board where that shows,
which is why two pieces never met it. A first repair, both orders of
the two groups, agreed on five of thirteen boards and allowed too much
on the rest, and was not kept. The repair made (9b74526, 4b34091,
95256e1): the damage's triggers are triggers like any other, set off
with the deaths' and ordered and resolved by `settle_triggers` in place
of the combat's own handling of them; with them on one stack, a trigger
whose target another has removed does nothing, and a token another has
made is there for a third to find. Against it the first five shares
all agreed, the 14 boards among them, and the sixth disagreed in 2 of
746,372 combats, on one board: defect 48, the engine's, and the block
triggers the reference resolved together. With both fixed (5a8c669,
a24dc8b) all sixty shares agree: 87,795,922 boards of the family and
44,778,274 combats, every way each can be declared and every question
in it answered every way. Two boards needed the check itself changed:
Falkenrath Marauders beside two Rakish Heirs, and three Rakish Heirs,
all attacking, give one player seven triggers to order, and nine, each
Heir triggering for each Vampire that deals its damage. Every order is
played (5,040, and 362,880), and the check now keeps one game for each
ending instead of one for each order. The fifteen boards that disagreed
are fought again in `cargo test`
(`combat_agrees_on_the_three_piece_boards_that_once_did_not`).

**Three pieces, the other checks, sampled.** One board in sixty (share 0
of 60: 1,463,395 boards of the family), which is what a share of this
machine's hours allows:

| Check | Plays | Result |
|---|---|---|
| end steps | 11,707,160 | all agree |
| werewolves | 17,560,740 | all agree |
| loyalty abilities | 16,097,345 | all agree, a search's shuffle played under a board's death trigger (284,184 more than the 15,813,161 before that cut was lifted, of which 8 had disagreed: one gap of the reference, what watches a death, above, the engine right) |
| upkeeps | 40,720,744 | all agree |
| a permanent entering beside a watcher | 8,780,370 | all agree |
| a spell cast beside a watcher of casts | 17,560,740 | all agree |
| flashback, one board in 480 | 6,668,805 | 1 disagreed, the gap of the shuffled library (the last above), the engine right; against it as fixed all agree |

Whole, on these four cores, end steps on three pieces are about 22
hours, upkeeps about 85 and the loyalty abilities about 60.

**Next.** The cuts above; and three pieces for the other checks. Combat
on three pieces was five hours of this machine. The casts are another
matter: 88 million boards and about 25 billion casts, some 500 hours on
these four cores, a day and a half on 64, which is what a larger machine
would be asked for.

### B2. Kani with the engine under a card (under way)

The pilot above concluded that Kani cannot hold the engine under a card.
Measured one freedom at a time (2026-11-18, `kani-bounded.py`, one harness
at a time, the peak memory of `cbmc` sampled), that was wrong about a
card's own hook and right about the stack:

| Harness | What is open | Result |
|---|---|---|
| Doom Blade's `on_resolve`, the real `try_destroy` and `move_object` | nothing | proves, 58 s, 1.12 GB |
| the same | one of: tapped, controller, damage, toughness 2 or 3, a shield, indestructible | proves, 58-68 s, 1.09-1.16 GB each |
| the same | all six at once | proves, 65 s, 1.22 GB |
| the same, the indestructible effect pushed only when it is there | all six | out of memory at 12 GB, 270 s |
| Doom Blade resolved by `resolve_top_of_stack` | nothing | out of memory at 12 GB, 555 s |
| Giant Growth's `on_resolve`, read back by `effective_power` and `effective_toughness` | controller, tapped, damage, toughness | proves, 51 s, 0.98 GB |
| Think Twice's `on_resolve`, the real `draw_cards` | which player, a library of 0, 1 or 2 | out of memory at 12 GB, 307-396 s each |
| the same, a harness for each player | a library of 0, 1 or 2 (a harness each) | proves, 25-60 s, 0.77-1.08 GB each |
| Midnight Haunting's `on_resolve`, the real token making | which player | proves, 107 s, 1.90 GB |
| Geistflame's `on_resolve`, the real damage, at a creature | controller, tapped, damage, toughness | proves, 71 s, 1.24 GB |
| the same, at a player | which player | proves, 56 s, 1.16 GB |

So a card's hook with the engine's own actions under it is about a
minute and one to two gigabytes, scalars open, and the two things that
put it out of reach are the two the engine's own proofs already name: a
list whose length the solver leaves open (the pilot's "lists fixed"
harness pushed its indestructible effect conditionally, which is what ran
it out of memory, not the scalars), and a player it leaves open where the
card reaches into that player's library or hand. The stack's resolution
of a spell is out of reach whatever is open, and stays the engine's proof
(`proofs::actions::resolving`) and the small boards' check.

**Where it stands (2026-11-19).** `proofs/engine_under_card.rs`: 595
harnesses over 266 of the 270 cards, every one proved: 202 over 86 of
the 88 spells, and 393 over 180 of the 182 permanents (below); 15.6
hours of solver time in all on this machine while it was busy, the
longest 12 minutes and most a minute or two. The spells, by family, each a
function of what the text says and a row for each card and case:

| Family | Cards | What is asserted of the board afterwards |
|---|---|---|
| "Destroy target ..." | Doom Blade, Victim of Night, Smite the Monstrous, Rebuke, Ancient Grudge, Naturalize, Urgent Exorcism, Bramblecrush, Maw of the Mire, Into the Maw of Hell, Paraselene | in its owner's graveyard and out of combat; untouched if indestructible; tapped, undamaged, out of combat and the shield spent if it regenerates; the life or damage the text adds |
| "deals N damage to ..." | Geistflame, Lightning Bolt, Lava Axe, Brimstone Volley (with and without morbid), Devil's Play (X of 0, 1, 4), Corpse Lunge, Harvest Pyre, Blasphemous Act | N more damage marked on a creature, N life fewer for a player, N loyalty fewer for a planeswalker, each amount a harness |
| pumps and until-end-of-turn effects | Giant Growth, Ranger's Guile, Moment of Heroism, Spidery Grasp, Travel Preparations, Feeling of Dread, Nightbird's Clutches, Rally the Peasants, Vampiric Fury, Hysterical Blindness, Traitorous Blood | power, toughness and keywords as the engine's layers read them back; each of up to two targets with none, one and two; each creature of the side named and no other |
| draws and mills | Think Twice, Divination, Altar's Reap, Dream Twist | the top N in the hand or graveyard in order, the rest in order, the empty-library draw marked; each player and each library size a harness |
| tokens | Midnight Haunting, Moan of the Unhallowed, Spider Spawning | that many tokens of that power, toughness, colour, subtype and keyword under the caster |
| Auras and Curses | the twelve "enchant creature" Auras and Wreath of Geists, the six Curses | attached, and the creature (or the enchanted player's creatures) as the text says, read back through the engine's layers and its attack, block and untap queries |
| moves between zones | Counterspell, Dissipate, Lost in the Mist, Silent Departure, Grasp of Phantoms, Swords to Plowshares, Purify the Grave, Runic Repetition, Unburial Rites, Ghoulcaller's Chant, Sever the Bloodline | where each card is, under whom, and what the list it left and the list it joined hold |
| the rest | Bump in the Night, Infernal Plunge, Gnaw to the Bone, Prey Upon, Rolling Temblor (one creature without flying), Tribute to Hunger | life, mana, a fight's damage, and a question asked and its effect applied |
| cards taken by what they are | Mulch, Past in Flames, Creeping Renaissance, Night Terrors, Forbidden Alchemy, Caravan Vigil (with and without morbid), Moonmist | each card where its kind sends it; the flashback the engine then reads; the question asked, of whom, among what, and what its answer does |
| at random, at a seed | Make a Wish, Desperate Ravings, Memory's Journey | how many cards moved and which stayed, with the engine's own shuffle and pick under the card |
| the last of the spells | Frightful Delusion, Cackling Counterpart, Divine Reckoning | the payment asked of the spell's controller and the discard after it; a token that is a copy, read back through the layers; the creature each player keeps and the rest destroyed |

Three more things that put a harness out of reach were measured on the
way: an amount of damage the solver leaves open (Brimstone Volley's 3 or
5 in one harness ran out of memory at 7 GB; a harness for each proves in
under two minutes); a board of seven objects under the default loop
bound of 7 (Spider Spawning with two creature cards: a table may say
`unwind(9)`); and a card that asks of each creature whether to act on it
before queueing what it does (Rolling Temblor's "without flying": more
than one permanent on the board, and it does not fit).

**A card that asks.** Tribute to Hunger is the first: its hook leaves a
question (who is asked, among what, whether they may decline, and the
effect an answer applies), and the engine's answering of a choice
(`engine::actions::choices::resolve_choice`) checks the answer is one of
the options and applies that effect. Measured: the hook alone, with the
real asking, proves in 72 s; the hook and then `resolve_choice` ran out
of memory at 5 GB on a board with nothing open, as the stack's
resolution does; and so did the hook and then the effect applied
directly, until the question was read where it lies rather than taken
out of the state (CBMC copies that enum whole). As it stands the proof
runs the hook, holds the question it left to the text, and applies the
effect the question holds through the engine's own
`apply_pending_effect` at each option: 145-151 s. The dispatcher between
is proved of itself (`proofs::actions::answering`). This is the shape
for every card that asks.

**The engine's dispatch of a card's effect, and then a read.** Fiend
Hunter's question holds an effect that is the card's own
(`PendingEffect::CardEffect`), and the engine applies it by finding the
card's behaviour in the registry from an id read back off the object.
Measured on a board with nothing open: the effect applied through that
dispatch and the creature then found in exile proves in 108 s; asked as
well which card the Hunter then holds, it ran out of memory at 7 GB,
with room given to the Hunter's list first or without; and the card's
own hook called as the engine calls it, and then asked the same, proves
in 80 s. The likely reason, which was not measured further: under the
dispatch the solver cannot tell which card's behaviour it was handed,
so what the card writes is written by either card of the registry or by
neither, and every later read of it is of all of those. So a harness
applies an effect through the engine where it then reads the board
(zones, life, damage), and calls the card's hook where it then reads
what the card keeps of its own: Fiend Hunter's exiled card (the whole
card hook to hook, enter to return, in 107 s, where through the
dispatch it did not finish symbolic execution in 25 minutes), Caravan
Vigil's land with morbid, Divine Reckoning's kept creatures.

**Not yet.** Rolling Temblor beside a creature that has flying, and
Rolling Temblor beside any second permanent (one creature without
flying, alone, proves in 71 s; each of the others ran out of memory at
8 GB, nothing about the board left open). Spare from Evil: with one
creature of yours and one other, the engine's protection read back ran
out of memory at 8 GB. These three are what more memory would be tried
on first. And Army of the Damned: thirteen tokens are more permanents
than a list of the engine's has room for in a proof here (below).

**Permanents.** 393 harnesses over 180 cards. For a card with a hook,
the hook as its trigger or ability resolves, for a source still on the
battlefield and, where the text does not need it there, for one gone
from it by then (a counter then lands nowhere, CR 121.1; a drain, a
draw, a token or damage happens all the same, for the player who last
controlled it, 608.2h). For a card with none, the card on the
battlefield and the board read back through the engine's own layers
and queries, or the event its text replaces run through the engine:

| What sets it off | Cards |
|---|---|
| entering | Armored Skaab, Claustrophobia, Crossway Vampire, Geist-Honored Monk, Hollowhenge Scavenger, Morkrut Banshee, Village Bell-Ringer, Woodland Sleuth; Slayer of the Wicked and Geistcatcher's Rig ask |
| a death, its own or one it watches | Doomed Traveler, Mausoleum Guard, Elder Cathar, Pitchburn Devils, Rage Thrower, Falkenrath Noble, Selhoff Occultist, Abattoir Ghoul, Galvanic Juggernaut, Gutter Grime, Lumberknot, Unruly Mob, Village Cannibals; Thraben Sentry and Murder of Crows ask |
| combat damage | Bloodcrazed Neonate, Falkenrath Marauders, Stromkirk Noble, Stromkirk Patrol, Rakish Heir, Curse of Stalked Prey, Sturmgeist, Balefire Dragon, Creepy Doll (its coin either way); Curiosity asks |
| an upkeep | the twelve Werewolves, each face; Splinterfright, Curse of the Bloody Tome, Bloodgift Demon, Angel of Flight Alabaster, Curse of the Pierced Heart and its question, Endless Ranks of the Dead; Cloistered Youth asks |
| an end step, a spell cast, a creature entering, a card milled | Unholy Fiend, Howlpack Alpha (gone), Reaper from the Abyss; Charmbreaker Devils, Burning Vengeance; Champion of the Parish; Undead Alchemist |
| an attack or a block | Ashmouth Hound, Wooden Stake, Hamlet Captain, Grimgrin, Kessig Cagebreakers (with nothing to count) |
| equip | the eleven Equipment, each read back through the layers, at a Human where the text says more of one |
| a hook that asks, searches, pays or picks | Fiend Hunter (the question, what it holds, its return, and the whole card), Witchbane Orb, Snapcaster Mage, Nevermore's question; Moldgraf Monstrosity, Bitterheart Witch; Delver of Secrets, Curse of Oblivion, Screeching Bat on either face and Mentor of the Meek paying from the pool; Homicidal Brute; Trepanation Blade; Ghoulraiser and Charmbreaker Devils' upkeep at two seeds; Olivia Voldaren's second, Mindshrieker, Heretic's Punishment, Graveyard Shovel, Civilized Scholar, Stitcher's Apprentice, Skeletal Grimace's regeneration, Blazing Torch's sacrifice, Brain Weevil, Traveler's Amulet, Ghost Quarter, Mirror-Mad Phantasm, Back from the Brink, Grimoire of the Dead (both) |
| a static ability, read through the layers | Glorious Anthem, Heartless Summoning's -1/-1, Intangible Virtue, Angelic Overseer, Night Revelers, Boneyard Wurm, Scourge of Geier Reach |
| a keyword, evasion or protection, asked of the engine | Elite Inquisitor, Invisible Stalker, Grave Bramble, Vampire Interloper, Orchard Spirit (against a creature that prints flying, one that prints reach and one that prints neither) |
| how it enters, put onto the battlefield by the engine's own zone change | Diregraf Ghoul; Clifftop Retreat, Hinterland Harbor, Isolated Chapel, Sulfur Falls and Woodland Cemetery beside each basic land type they name and beside the other player's; Festerhide Boar and Somberwald Spider with and without a death; Unbreathing Horde; Dearly Departed from a graveyard; Essence of the Wild's copy |
| an event it replaces, run through the engine | Laboratory Maniac under the engine's draw, Parallel Lives under its token making, Unbreathing Horde under its damage, Stony Silence |
| a mana ability activated through the engine | Forest, Island, Mountain, Plains, Swamp, Avacyn's Pilgrim, Sol Ring, Shimmering Grotto's first |
| the creature it prints, the numbers and keywords the Oracle text's | the nineteen creatures with no ability or keywords alone; and the body of Skaab Goliath, Makeshift Mauler, Stitched Drake, Skaab Ruinator, Battleground Geist, Gallows Warden and Geist of Saint Traft, whose own ability is the engine's or is proved elsewhere (below) |
| a loyalty ability | Garruk Relentless (the fight, the Wolf, the transformation at two loyalty or fewer) and Garruk, the Veil-Cursed (the Wolf, the sacrifice and search, the last); Liliana of the Veil (each player's discard, the sacrifice, the piles asked for) |
| an activated ability that asks nothing | Darkthicket Wolf, Feral Ridgewolf, Kessig Wolf, Avacynian Priest, Bloodline Keeper (both), Skirsdag High Priest, Nephalia Drownyard, Ghoulcaller's Bell, Stensia Bloodhall, Lantern Spirit, Elder of Laurels, Gavony Township, Manor Skeleton, Ulvenwald Primordials, Manor Gargoyle, Mikaeus (both), Selfless Cathar, Silverchase Fox, Skirsdag Cultist, Daybreak Ranger's damage and Nightfall Predator's fight, Olivia Voldaren's first, Kessig Wolf Run, Disciple of Griselbrand, Tree of Redemption, Deranged Assistant, Cellar Door, Ludevic's Test Subject, Full Moon's Rise, Grimgrin, Moorland Haunt, Evil Twin |

Nine more things that put a harness out of reach, each measured:

- A zone change for an owner the solver leaves open is a graveyard it
  leaves open (4 GB): a source that has left is one player's card under
  either player.
- A value the solver can prove constant is not one symbolic execution
  has folded. A Werewolf's face, just written by the transform, read by
  the layers: the hook alone proves in a minute, the read alone in a
  minute, the read after the hook ran out of memory at 7 GB. The harness
  asserts the face and writes it back as that constant, which changes
  nothing on any run the assertion allows, and proves in 90 seconds.
- A filter read back from a card's printed effects is one the solver
  matches as every filter there is (`proofs::actions::filters` says so
  of the engine's own). Instigator Gang's "attacking creatures you
  control" ran out of memory at 5 GB with nothing open and no trigger
  run: its harnesses assert the transform and leave the power unread,
  and that filter is now proved of `matches_filter` beside the five that
  were. Howlpack Alpha's Wolf is proved with the Alpha gone, and not
  beside it.
- Five permanents on the battlefield: a list the engine collects them
  into outgrows its first allocation, which a proof here counts a
  failure. Endless Ranks of the Dead is proved on boards of four.
- A token put onto the battlefield attacking (Geist of Saint Traft's
  Angel, Kessig Cagebreakers' Wolves): out of memory at 7 GB. The
  engine's own proofs take it in two halves.
- A card picked at random and then moved, the random numbers left open:
  the card moved is one the solver leaves open (Woodland Sleuth, 7 GB).
  A harness for each of two states of the random numbers, one picking
  the first card and one the second, proves in two minutes with the
  engine's own shuffle under it. A coin with the numbers open fits
  (Creepy Doll, both ways reached).

- A filter a card builds of several questions, read back from the card.
  "Creatures you control" (Glorious Anthem) and "creature tokens you
  control" (Intangible Virtue) are one question and prove in a minute
  or two. "Other Spirit creatures you control" (Battleground Geist,
  Gallows Warden) and "creatures with flying your opponents control"
  (One-Eyed Scarecrow) are a list of two, and each ran out of memory at
  7 GB in symbolic execution on a board of the card and one creature
  with nothing open; the Scarecrow alone did, its filter asking of the
  Scarecrow itself whether it flies as its own power is read. What each
  of those filters matches is proved of the engine with the filter built
  where it is matched (`proofs::actions::filters`, eight filters now),
  and that the card prints that filter is its declared harness.
- A keyword is what a card prints, not something written on a
  permanent. A harness gave a blank creature flying by writing the
  keyword onto it and claimed it could block an Orchard Spirit; the
  proof failed, and the engine was right (`GameState::scan_keywords`
  reads a card's keywords from its face, so that a transformed
  permanent keeps none of its front's). The harness now uses a creature
  that prints flying and one that prints reach.
- A player left open, again, for a mana ability: a Plains tapped
  through the engine for either player proved alone under 7 GB and the
  other four basic lands and Sol Ring ran out of memory there; for the
  first player each proves in under a minute.

Not yet, of permanents: those just named (Instigator Gang's power read,
Howlpack Alpha beside its Wolf, a board of five, Geist of Saint Traft's
Angel and Kessig Cagebreakers' Wolves); a payment planned from lands
under a card's "may pay", which is the engine's and proved of it
(`proofs::actions::paying`), the card proved here with the mana
floating; and what an answer does where the engine and not the card
does it (the card Forbidden Alchemy's chooser takes, the name Nevermore
is given, Liliana's piles, a discard chosen from a hand), each proved of
the engine's own answer (`proofs::actions`).

Of the 270 cards, 266 have a harness with the engine under them. The
four that do not: Army of the Damned (thirteen tokens, more permanents
than a list of the engine's has room for in a proof here), Spare from
Evil (the protection read back, above), One-Eyed Scarecrow (its filter,
above) and Rooftop Storm, whose whole text is another cost for a spell,
which is the engine's cast and not a hook of the card's or a board to
read. And for some the harness is of part of the text, the rest being
the engine's own path: the additional cost of Skaab Goliath, Makeshift
Mauler, Stitched Drake and Skaab Ruinator, and the Ruinator's cast from
a graveyard; Heartless Summoning's cheaper spells; Shimmering Grotto's
second ability; the lord effects of Battleground Geist and Gallows
Warden (their filter, proved of the engine); Geist of Saint Traft's
Angel; Instigator Gang's bonus. All 270 are played with the engine on
every small board by `tests/card_semantics.rs`.

These harnesses are also the first proofs of the engine's actions
composed with nothing stood in for: `try_destroy` over the real
`move_object` and `has_keyword`, the damage queue over the real
settling, `draw_cards` over the real replacement lookup. That is part of
what B3 asks for.

### B3. The engine's own proofs

Each engine proof still stands in for the functions its function calls.
Those stand-ins are not between a card and the game, but the objection
is the same in kind: nothing proves a function behaves as its recorder
says. Closing it is a proof, for each recorder, that the real function's
effect is what the recorder is assumed to have (or, where the recorder
only records, that every proof using it asserts nothing about state the
real function would have changed).

## What is reported

`reports/kani-card-ledger.md` lists the Kani card harnesses that are
kept. A ledger of the
reference's coverage follows A2: for each card and each of its abilities,
the tests that run it with the engine, on how many boards, and what of its
text the reference does not yet read.
