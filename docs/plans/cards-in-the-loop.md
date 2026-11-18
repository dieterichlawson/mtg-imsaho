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

Every one of those agrees with its card's text. They run in
`scripts/prove.sh exhaustive`, and so on every push and pull request.
`CARD_SEMANTICS_SMALL=2` is every board of two pieces: 327,645 boards,
about 91 million casts and 18 million activations, some six hours for the
five on four cores, run in shares (`CARD_SEMANTICS_SHARD=K/N`); a failure
names its board, and `CARD_SEMANTICS_SMALL_BOARD=I,J` replays it alone.

**What it has found so far.** No engine defect yet; seven gaps of the
reference, each on a board nobody wrote and in each the engine right:

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
  pieces).

And what the checks had taken from their hand-made boards and no longer
do: a board's own triggers, set off by a cost or a resolution, settled
every way in the activated and flashback checks as they were in the
cast check; the opponent's spell held on the stack cast on any creature
of theirs; the source "exiled in response" played only where the
ability is alone on the stack.

One cut, where the reference does not yet read far enough: a loyalty
ability that searches a library shuffles it, and what a death trigger of
the board's then mills or draws is any card of it, which the reference
does not play; such an ability (Garruk, the Veil-Cursed's sacrifice) is
not played beside a permanent that triggers on a death.

**Next.** The two-piece run to its end, each disagreement worked to its
cause. Then the properties still on hand-made boards alone: the
watchers of casts, and combat; and the cut above. Three pieces as they entered is 88 million boards
and about 25 billion casts: some 500 hours on these four cores, a day
and a half on 64, which is what a larger machine would be asked for.

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

`proofs/engine_under_card.rs` holds these, eleven harnesses over five
cards, all proved. What they take out is `format!`, the log, and growing
and freeing an allocation; no engine action. Next, family by family over
the 88 spells (the eight "destroy target", the five "deals N damage",
the Auras and Curses attaching, the draws, the tokens, the pumps), then
the hooks of permanents: each row a harness, each about a minute, so the
whole of it is hours on the full tier and not on a pull request's.

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
