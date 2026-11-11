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
  in for. Of the 40 defects found so far, 3 were found by card proofs.
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

That settles the split:

- **The engine in the loop is the reference's job**, on boards and in
  situations drawn at scale rather than written by hand (part A).
- **The Kani card proofs stay compositional**, and are made sound about the
  engine instead of merely consistent with themselves (part B).

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

### A2. Wider boards (next)

The vocabulary leaves out, each for a reason the reference does not yet
carry to a board it did not write:

1. a permanent with a dies trigger or a watcher of deaths, casts or
   entries: the reference fires these for the card under test, and must
   for any permanent on the board (a Doom Blade killing a Doomed Traveler,
   a Murder of Crows watching it), the way it now does for entries;
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

`cargo test` runs four boards per card (about 20 s in a debug build). The
exhaustive tier runs hundreds, in release, sharded with
`CARD_SEMANTICS_FIRST_BOARD`.

## Part B: Kani card proofs that are sound about the engine

1. **Stand-ins that do what the action does.** Each recorder becomes a
   model of its action's specification: it changes the state as the
   action's proof says the action does, and where the action may fail or
   choose, it does each nondeterministically. Code after the call then
   reads the state the call left, and every branch on its result is taken.
2. **Each action proved to refine its stand-in**, on the same bounded
   boards: the action proofs in `proofs/actions/` restated as "the real
   function leaves what the stand-in may leave". Then a card proof with
   the stand-in holds of the card with the real action, within the bounds.
3. **Outcomes, not calls.** Each card's assertion is restated as the state
   its resolution leaves, written from its Oracle text: the destroyed
   permanent in its owner's graveyard, not a `Destroy` in the record.

This is a rewrite of the 990 harnesses and the 51 stand-ins, and is done
after A2, family by family, the families with the most cards first.

## What is reported

`reports/kani-card-ledger.md` lists the Kani harnesses. A ledger of the
reference's coverage follows A2: for each card and each of its abilities,
the tests that run it with the engine, on how many boards, and what of its
text the reference does not yet read.
