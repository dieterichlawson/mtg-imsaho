# What the proofs catch that the tests do not — 2026-10-25

`mtg-kernel` holds the pieces of the rules proved with Verus: paying a mana
cost out of a pool (`mana.rs`), dividing a blocked attacker's combat damage
(`combat.rs`), and, measured separately below, funding X (`funding.rs`). This is a measurement of what those
proofs add over the test suite. It follows Anthropic's post on LLM-driven
formal verification (2026-10-22), which injected seven bugs into a verified
Python interpreter: five survived the random tests, all seven broke the
proof.

## Method

29 mutants, one or more for every executable statement of the two kernels:
an operator flipped, a floor or an order replaced, a loop bound one short,
an answer ignored (`scripts/kernel-mutants.py` lists them, and re-runs this).
cargo-mutants cannot make these, because it does not mutate inside macro
bodies and everything in `mtg-kernel` is inside `verus!`. Every mutant
compiles. For each one:

- **proof**: does `cargo verus` reject the mutated kernel?
- **tests**: do the tests fail: `mtg-kernel`'s own, `mtg-engine`'s unit
  tests, and the twelve integration test files that exercise payment, X
  funding and combat? For any mutant those miss, the whole `mtg-engine`
  suite was run as well.

The tests are the suite as it stands after today's work, including the
regression tests added alongside the proofs (the three payment defects, the
X-funding defect, the combat-division completeness check, and the test that
holds `surplus_order` to the `sort_by_key` it replaced). Before today, the
suite missed at least the four defects those tests were written for.

## Results

| mutant | what it breaks | proof | tests |
|---|---|---|---|
| M01 | `pay_fixed` ignores the generic part | caught | caught |
| M02 | `pay_fixed` accepts being one short | caught | caught |
| M03 | `pay_around` spends spare mana in slot order, not surplus order | missed | missed (whole suite too) |
| M04 | `pay_around` takes all of a type first (payment defect 2) | caught | caught |
| M05 | `pay_around` never digs into the reserve | caught | caught |
| M06 | `pay_around` forgives one point | caught | caught |
| M07 | `surplus_order` is not stable | missed | caught |
| M08 | `surplus_order` sorts smallest surplus first | missed | caught |
| M09 | `surplus_order`'s swap duplicates a slot | caught | caught |
| M10 | the surplus adds the reserve instead of subtracting it | missed | caught |
| M11 | `surplus_order` never places the last slot | missed | caught |
| M12 | a type with exactly its pips is called short | caught | caught |
| M13 | a type pays at most one of its pips | caught | caught |
| M14 | green pips are never paid | caught | caught |
| M15 | spending generic ignores the floor | caught | caught |
| M16 | spending generic takes at most one of each type | caught | caught |
| M17 | spending generic never reaches the last slot | caught | caught |
| C01 | the least a blocker gets ignores how much is left | caught | caught |
| C02 | a trampler's last blocker takes everything | caught | caught |
| C03 | only a trampler's last blocker takes the rest | caught | caught |
| C04 | the first blocker is never asked about | caught | caught |
| C05 | an answer below lethal is taken | caught | **missed** (whole suite too) |
| C06 | the player's answer is ignored | caught | caught |
| C07 | a later blocker's question replaces the first | caught | **missed** (whole suite too) |
| C08 | the range stops one short of everything left | caught | caught |
| C09 | a non-trampler's leftover goes past its blockers | caught | caught |
| C10 | trample loses a point | caught | caught |
| C11 | a choice of exactly two amounts is not asked | caught | caught |
| C12 | lethal is counted one short | caught | caught |

The proof caught 24 of 29 and the tests 26 of 29. Each caught mutants the
other missed.

**Only the proof (C05, C07).** C07 is the instructive one: if a later
blocker's question replaces the first, the attacking player is asked about
blockers out of order, and an amount they gave for a later blocker can be
silently cut when they then answer an earlier one. The exhaustive test
(`every_division_obeys_cr_510_1c`) checks every answer sequence on every
small board, and still passes. It checks that the *set* of divisions the
prompts reach is the set of legal ones, and that set does not change. The
proof's statement is per question: the answers given so far are kept, and
the next question is about the first blocker still open. C05 is defence in
depth: the engine refuses an out-of-range answer before it reaches the
kernel (`an_amount_outside_the_range_is_refused`), so no test can deliver
one, but the kernel's contract says a division is legal whatever it is
handed, and the proof holds it to that.

**Only the tests (M07, M08, M10, M11).** These all change *which* spare
mana is spent first, not whether a payment succeeds or keeps the reserve.
That order is a preference ("what another cost depends on most goes last"),
and the proof deliberately does not specify it. It asks only that the order
names every slot once, because the payment properties hold for any order.
The unit test that holds `surplus_order` to `sort_by_key` over 531,441
pools is what guards the preference.

**Neither (M03).** Spending spare mana in slot order rather than by surplus
breaks no rule and no promise either suite states: the reserve is still
kept. It is the same preference as above, at the call site. It is left
unpinned on purpose (`docs/mutation-testing-guide.md`, "would my test still
pass if this function were rewritten correctly a different way?"); if the
preference ever matters to a player, the property to state is "spare is
spent biggest surplus first", and it belongs in the proof.

## The funding kernel — 2026-10-26

`funding.rs` was proved after the first measurement, and measured the same
way: 14 mutants, F01–F14 in `scripts/kernel-mutants.py`, with the page's
X-funding fixture test (`gui_protocol`, 168 boards) added to the tests run.

| mutant | what it breaks | proof | tests |
|---|---|---|---|
| F01 | a group's last source is never tapped | caught | caught |
| F02 | a group has as many sources as it likes | caught | caught |
| F03 | a group has one source more than it does | caught | caught |
| F04 | one tap more than a reachable sum is missed when it starts from nothing | caught | caught |
| F05 | tapping nothing is not a way to pay nothing | caught | caught |
| F06 | the pool only tops up what taps cannot pay exactly | caught | caught |
| F07 | a tap sum the whole pool tops up exactly is passed over | caught | caught |
| F08 | a short X stops looking below one | caught | caught |
| F09 | a short X leaves a point in the pool | caught | **missed** (whole suite too), then caught |
| F10 | the first group is never tapped | caught | caught |
| F11 | a group takes the most it can whether or not the rest can finish | caught | caught |
| F12 | a group holds back its last source | caught | caught |
| F13 | a fundable X is measured from the smallest tap sum, not the nearest | caught | caught |
| F14 | the whole pool never counts towards a fundable X | caught | caught |

The proof caught all 14 and the tests 13. F09 changes how much the kernel
says comes from the pool when X cannot be paid in full, and the engine
never read that number: it drained the pool by its own loop. So no test
could see it, and the proof could, because the contract states it. The
engine now asserts its drain against the kernel's amounts in debug builds
(`allocate_for_x`), which is what turned F09 to caught: the glue is held
to the proof rather than trusted beside it.

The pattern the two measurements share: the proofs catch what the
contracts state, whether or not anything downstream looks (C05, C07, F09),
and the tests catch what the code is preferred to do beyond the contract
(M07, M08, M10, M11). A kernel output nobody reads is a place the two can
drift apart, and a debug assertion at the call site closes it.

## The turn, state-based-action and block kernels — 2026-10-26

Each kernel was broken four ways by hand, one line at a time, and Verus
run on each break. Every break is rejected.

| kernel | break | proof |
|---|---|---|
| `turn` | no attackers still goes on to combat damage | rejected |
| `turn` | the first turn's draw step is not skipped | rejected |
| `turn` | a cleanup step that does not repeat goes on to an untap step | rejected |
| `turn` | priority needs one pass more than there are players | rejected |
| `sba` | toughness 0 survives | rejected |
| `sba` | deathtouch kills a creature dealt no damage | rejected |
| `sba` | no counter pairs are removed | rejected |
| `sba` | lethal damage needs one more point | rejected |
| `blocks` | an attacker's minimum is short by one | rejected |
| `blocks` | a block the rules refuse is let through | rejected |
| `blocks` | a blocker may block a second attacker | rejected |
| `blocks` | a block is miscounted towards its attacker's minimum | rejected |

## The cards, family by family — 2026-10-26

Each new family of `card_semantics.rs` was checked by breaking the engine
in a card that uses it, or in the engine code under it, and running the
whole file. 48 mutants; 47 caught.

| family | mutant | caught by |
|---|---|---|
| mana abilities | Sol Ring makes one | the mana test |
| | Deranged Assistant mills nothing | the mana test |
| | summoning sickness ignored for mana | the mana test |
| combat text | Stromkirk Noble blockable by Humans | the fight test |
| | Grave Bramble protected from Vampires | the fight test |
| | Falkenrath Marauders takes one counter | the fight test |
| | Night Revelers' haste keyed to Elves | the fight test |
| | Sturmgeist draws nothing | the fight test |
| | Vampire Interloper can block | the fight test |
| | Bloodcrazed Neonate not forced to attack | the fight test |
| | Invisible Stalker blockable | the fight test |
| | Balefire Dragon deals half | the fight test, once the defender also declines to block |
| statics and lords | Battleground Geist gives +2 | the cast test and the standing-by test |
| | Battleground Geist pumps itself | the cast test and the fight test |
| | Battleground Geist pumps the opponent's Spirits | the cast test and the fight test |
| | Gallows Warden gives nothing | the cast test and the standing-by test |
| | One-Eyed Scarecrow shrinks its own fliers | the cast test and the fight test |
| | One-Eyed Scarecrow shrinks every opposing creature | the cast test and the fight test |
| | Scourge of Geier Reach counts its own creatures | the cast test and the fight test |
| | Angelic Overseer without indestructible | the cast test and the standing-by test |
| | the engine's "other" includes the source | the cast test and the fight test |
| | state-based actions ignore indestructible | **only** the standing-by test |
| other deaths | Rage Thrower deals 1 | the cast test |
| | Unruly Mob watches the opponent's creatures | the cast test |
| | Village Cannibals watch Elves | the cast test |
| | Falkenrath Noble's own death drains nothing | the cast test |
| | Selhoff Occultist mills two | the cast test |
| | Lumberknot takes two counters | the cast test |
| | a watcher sees its own death | the cast test |
| | a watcher that dies in the same batch does not look back | the cast test, **only** through two deaths at once |
| upkeep | Bloodgift Demon drains 2 | the upkeep test |
| | Angel of Flight Alabaster returns any card | the upkeep test |
| | Splinterfright mills one | the upkeep test |
| | Curse of the Bloody Tome mills three | the upkeep test |
| | Curse of the Pierced Heart deals 2 | the upkeep test |
| | Curse of Oblivion exiles one | the upkeep test |
| | "your upkeep" fires every upkeep | the upkeep test, in the other player's turn |
| | a Curse fires in its controller's upkeep | the upkeep test, in the other player's turn |
| | the untap step untaps every player's permanents | the upkeep test, with the lands tapped |
| Equipment | Butcher's Cleaver gives +2 | the activated-ability and standing-by tests |
| | Butcher's Cleaver's lifelink for Elves | the activated-ability and standing-by tests |
| | Cobbled Wings give reach | the activated-ability and standing-by tests |
| | Mask of Avacyn gives +1/+1 | the activated-ability and standing-by tests |
| | Sharpened Pitchfork without first strike | the activated-ability and cast tests |
| | Runechanter's Pike counts creature cards | the activated-ability test |
| | equip at instant speed | the activated-ability test's timing check |
| | a fallen Equipment goes to the graveyard | the standing-by test |
| | `equip_target_is_legal` accepts another player's creature | **missed** |

The survivor is reachable only through the re-check at resolution (CR
608.2b): the targets the engine offers come from the ability's target
requirement, not from that function, so it matters only when the creature
changes control between activation and resolution. No card the grammar
reads changes control yet; Traitorous Blood will.

Three mutants were caught only by a check added for the family: a
creature standing by out of combat, two deaths at once, and the other
player's turn. Each of those checks was added because the family's text
named a case the first version did not reach, and each then caught a
break nothing else did. The reading for later families: a test is chosen
from the text's cases, not from the engine's code paths.

## The cards, family by family, continued — 2026-10-26

The same method for the families after the first 48: 52 mutants, 51
caught and one equivalent.

| family | mutant | caught by |
|---|---|---|
| restricting Auras | Pacifism lets it block | the Aura fights |
| | Furor of the Bitten does not force the attack | the Aura fights |
| | Bonds of Faith stops Humans blocking | the Aura fights |
| | Ghostly Possession prevents nothing | the Aura fights |
| | Claustrophobia lets it untap | the turn test |
| | Claustrophobia does not tap | the cast test |
| | Curse of the Nightly Hunt forces the wrong player | the Aura fights |
| | the engine prevents combat damage only to the creature | the Aura fights |
| | the engine prevents combat damage only by the creature | the Aura fights |
| X | Devil's Play deals X-1 | the cast test |
| | Harvest Pyre deals 2X | the cast test |
| | Kessig Wolf Run gives +X/+X | the activated-ability test |
| | the engine records a spell's X one more than paid | the cast test |
| | the engine halves an ability's X | the activated-ability test |
| the mana pool | Brimstone Volley costs one less | the cast test, **only** once the pool is observed |
| | Grizzled Outcasts costs one less | the cast test, **only** once the pool is observed |
| | Kessig Wolf Run's ability costs no {G} | the activated-ability test |
| ability costs | Disciple of Griselbrand gains one more | the activated-ability test |
| | Skirsdag Cultist cannot sacrifice itself | the activated-ability test |
| | Demonmail Hauberk gives +4/+1 | the activated-ability test |
| | Moorland Haunt exiles any card | the activated-ability test |
| | Moorland Haunt's Spirit cannot fly | the activated-ability test |
| | equip's sacrifice is "another creature" | **equivalent**: an Equipment is not a creature |
| regeneration | Manor Skeleton gets no shield | the activated-ability test |
| | Full Moon's Rise shields the opponent's Werewolves | the activated-ability test |
| | regeneration does not tap | the lethal-damage follow-up |
| | regeneration leaves the damage | the lethal-damage follow-up |
| | state-based actions need two shields | the lethal-damage follow-up |
| | a destroy effect needs two shields | **only** the destroyed-by-an-effect follow-up |
| others' choices | Tribute to Hunger gains the power | the cast test |
| | Tribute to Hunger offers the opponent one creature | the cast test |
| | Divine Reckoning lets only the active player keep one | the cast test |
| | Divine Reckoning destroys the creature chosen | the cast test |
| what lasts the turn | "until end of turn" never ends | **only** the next-turn check |
| | damage stays past the turn | **only** the next-turn check |
| | regeneration shields last past the turn | **only** the next-turn check, for abilities |
| at random | Make a Wish returns one card | the cast test |
| | Moldgraf Monstrosity returns the first two, not two at random | **only** the seeds' completeness |
| | Moldgraf Monstrosity stays in the graveyard | the dies check |
| | Desperate Ravings draws one | the cast test |
| | the engine's random choice is always the first | **only** the seeds' completeness |
| searches | Caravan Vigil's battlefield option without a death | the cast test |
| | Traveler's Amulet puts the land onto the battlefield | the activated-ability test |
| | Ghost Quarter's land enters tapped | the activated-ability test |
| | Ghost Quarter's search is the activator's | the activated-ability test |
| combat triggers | Ashmouth Hound deals 2 | the fight test |
| | Geist of Saint Traft's Angel is a 3/4 | the fight test |
| | Geist of Saint Traft's Angel stays after combat | the fight test |
| | Kessig Cagebreakers makes one Wolf | the fight test |
| | Wooden Stake destroys any blocker | the Equipment fights |
| | Rakish Heir counts only itself | the standing-by fights |
| | Curse of Stalked Prey gives two counters | the Aura fights |

Nine of the 51 are caught only by a check added for their family, and
four of those checks are not about one card at all: the pool observed
after a cast, the next turn, the second way to destroy, and the seeds
having to reach every way "at random" allows. Each of the four found a
gap that every card already read was passing through: a spell could cost
less than its text, an "until end of turn" effect could last forever, and
an engine with no randomness at all would have passed every random card.
The checks were written from the rules the text invokes, not from the
engine's code, which is why they reach what no card-specific test did.

While writing these, the reference was wrong twice and the engine right:
two Rakish Heirs each trigger, and Tribute to Hunger's "that creature" is
the creature it made its controller sacrifice. Each was a fix to the
reference, made after reading the rule, and both are now regression
cases in the fights and casts.


## The cards: double-faced cards, end steps, and small templates — 2026-10-26

The same method for the families since: 45 mutants, 44 caught and one
equivalent.

| family | mutant | caught by |
|---|---|---|
| small templates | Elder Cathar gives a Human one counter | the dies check |
| | Elder of Laurels counts one more | the activated-ability test |
| | Infernal Plunge adds {R}{R} | the cast test |
| | Sever the Bloodline exiles only the target | **only** the board with two of a name |
| | Graveyard Shovel gains 3 | the activated-ability test |
| | Brain Weevil discards nothing | the activated-ability test |
| | Cellar Door takes the top card | the activated-ability test |
| | Tree of Redemption's toughness one more than the life it took | the activated-ability test |
| werewolf back faces | Bane of Hanweir is not forced to attack | the fights |
| | Wildblood Pack gives +2/+0 | the fights |
| | Instigator Gang pumps creatures that are not attacking | the fights |
| | Terror of Kruin Pass gives every creature menace | the fights |
| control change | Traitorous Blood does not untap | **only** the board with a tapped creature |
| | Olivia does not make it a Vampire | the activated-ability test |
| | Olivia's counter goes on the target | the activated-ability test |
| | the cleanup keeps control taken until end of turn | **only** the next-turn check |
| any transform | Thraben Sentry watches every player's creatures | the dies check |
| | Thraben Sentry never transforms | the dies check |
| | a second Sentry trigger transforms it back | **only** every "may" taken |
| | declining transforms it too | the dies check |
| | Thraben Militia lacks trample | the dies check |
| | Thraben Militia is 4/4 | the dies check |
| back faces' abilities | Ulvenwald Primordials do not regenerate | the activated-ability test |
| | Ulvenwald Mystics offer the Primordials' regeneration | **only** nothing beyond its text |
| | the engine's fight is one-sided | the activated-ability test |
| | Daybreak Ranger targets any creature | the activated-ability test |
| | Nightfall Predator's fight is free | the activated-ability test |
| | Daybreak Ranger reads the face up at resolution | **only** a flip in response |
| end steps | Unholy Fiend loses 2 | the end-step test |
| | Unholy Fiend's trigger on every end step | the end-step test |
| | Reaper from the Abyss triggers with no death | **only** the stack count |
| | Reaper's own check lets Demons in | **equivalent**: its target requirement says the same |
| | Reaper's requirement and check both let Demons in | the end-step test |
| | Cloistered Youth transforms when declined | the turn test |
| Mayor of Avabruck | Howlpack Alpha leaves out Wolves | the end-step test, the fights |
| | Howlpack Alpha pumps itself | the fights, the werewolf test |
| | the Mayor pumps the opponent's Humans | the cast test |
| | Howlpack Alpha's Wolf on every end step | the end-step test |
| | the Wolf token is a Werewolf too | the end-step test |
| Bloodline Keeper | four Vampires are enough | **only** one short of enough |
| | every permanent counts as a Vampire | the activated-ability test |
| | Lord of Lineage pumps itself | a flip in response |
| | Lord of Lineage pumps the opponent's Vampires | **only** the opponent's Vampire on the board |
| | Lord of Lineage offers the Keeper's transform | **only** nothing beyond its text, with either face's conditions met |
| | the engine drops CR 701.27f | a flip in response |

Six mutants survived when first run, and each became a check or a board
that every card read now passes through: a "you may" taken every time and
declined every time, since a set of endings can hide a wrong answer to one
behind a right answer to another; the number of things put on the stack,
since a trigger whose "if" is false must not trigger at all; each
permanent offering nothing its text does not print, with what either
face asks for met; an "if you control N" tried one short; and a Vampire
on the opponent's side. None was written for its card.

Two engine defects were found in the round, both by the reference
reading a rule the engine did not: CR 701.27f, under which an ability
does not transform its source if it has transformed since the ability
went on the stack (a test asserted the opposite), and CR 113.7a, under
which Daybreak Ranger's ability is the face's it was activated from.
Both are fixed, with regression tests that fail without the fix.
