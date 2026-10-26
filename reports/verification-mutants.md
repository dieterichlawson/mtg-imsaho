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

