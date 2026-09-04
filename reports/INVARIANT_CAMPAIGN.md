# The invariant campaign

A sweep of the Comprehensive Rules and the engine looking for properties
that hold at every decision point of every legal game, so that the fuzzer
can report an engine bug without anyone having to judge a card's behaviour.
This is the record of what came out of it.

## What the oracle is now

`mtg-engine/src/invariants/` — 4,900 lines, ~640 distinct violation
messages, in four tiers:

| Tier | Entry point | Holds at |
|---|---|---|
| core | `check_core(state, registry)` | every decision point, mid-resolution prompts and deserialized saves included |
| settled | `check_settled(state, registry)` | priority and turn-based-action prompts, where state-based actions have reached a fixed point |
| transition | `check_transition(prev, action, cur, registry)` | consecutive decision points and the action between them |
| legal | `check_legal(state, acting, legal, registry)` | the action set the engine offered |

The files: `objects.rs` (per-object shape and the card-code contract),
`stack.rs` (the stack, trigger queues, cast-time stashes), `prompts.rs`
(every prompt's shape and options), `turn.rs` (turn structure, the result,
combat bookkeeping), `events.rs` (the current action's event window),
`effects.rs` (delayed and continuous records), `permanents.rs`
(attachments), `transition.rs`, `legal.rs`, and `mod.rs` for the rest.

Both hosts run all four: `mtg-runner --check-invariants` (what the nightly
fuzz uses) and `mtg-player/tests/fuzz_random_games.rs`.

## Engine bugs it found

Every one has a regression test named after the rule it broke.

| Bug | Rule |
|---|---|
| Leaving the battlefield kept activation memory, copied types and keywords, the legendary flag, and control effects | CR 400.7 |
| A permanent that entered as a copy of a transformed permanent came back untransformed | CR 707.8 |
| An Evil Twin copy that died and returned was still a copy | CR 400.7 |
| Enters-as-copy left the legendary flag from the printed card | CR 704.5j |
| The legend-rule loser went to the graveyard without dying | CR 700.4 |
| An ability activated by sacrificing its own source was filed under the owner | CR 602.2a |
| An ability whose cost removed its source lost its target requirement | CR 601.2c |
| Forced attackers were missing from the declaration event | CR 508.1d |
| Cleanup did not repeat after a state-based action fired | CR 514.3a |
| Priority went to the wrong player after a cast-time prompt | CR 117.3c |
| An until-end-of-turn effect outlived the permanent it was on | CR 400.7 |
| A token that entered attacking could not be blocked | CR 509.1 |
| Destroying a creature that had already died announced a second death | CR 701.7a |
| A game won by Laboratory Maniac was never announced as ended | CR 104.2 |
| An Evil Twin entering as an Essence of the Wild stayed exempt from state-based actions | CR 614.1d |
| The legend rule grouped by the cached name, not the face that is up | CR 704.5j |

## How the checks were found

Six rounds, each a different way of looking:

1. **The rulebook.** The Comprehensive Rules read in slices (100s, 200-300s,
   400-500s, 600s, 700-701, 702, 703-724), asking of each rule: does this
   imply something about `GameState` that must hold card-independently?
2. **The engine.** Every field of `GameState`, `GameObject`, `PlayerState`
   and the stack read against its writers.
3. **The card code.** The contracts every card hook is implicitly held to —
   what a hook may leave behind, which fields cards write directly, and
   where two cards do the same thing differently.
4. **Time.** Properties of the *pair* of consecutive states: what may not
   change, what must, and the event buffer as a ledger explaining the delta.
5. **The offer.** The legal action set: nothing the rules forbid on the
   menu, and nothing they require missing from it.
6. **Mutation testing.** Twenty single-site engine bugs planted one at a
   time to measure what the oracle actually catches, plus adversarial
   soundness audits of every check against the engine that writes its
   fields.

Round 6 turned up the structural finding that mattered most: the game loop
passes on a player's behalf when they have nothing to do, and each of those
passes cleared the event buffer, so an observer watching only real decisions
never saw a combat-damage window at all. Under the checker those passes now
go through the decision callback, which turned several checks from
theoretical into effective.

## The bar

Zero false positives. A violation files a GitHub issue from a nightly run
over ~110k random games, so a check that fires on any legal sequence is
worse than no check. Every family carries a self-test in
`mtg-engine/tests/invariant_families.rs` that both flags the exact
corruption it claims to catch and passes the healthy version, and every
batch was validated by seeded fuzz bursts across all deck pairings before
being committed.
