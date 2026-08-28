## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/55/forbidden-alchemy?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "**Look at** the top four cards" — no reveal, and the code does not emit one.
- One card is auto-selected when only one is available, since there is no choice
  to present; two or more prompts the player.
- The rest go to the graveyard, not back on the library.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/55/forbidden-alchemy?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Look at the top four cards of your library. Put one of them into your hand and
  **the rest into your graveyard**." The rest are a library-to-graveyard move, so
  they emit `CreatureCardMilled` — they used to be moved by hand in the shared
  `ChooseFromRevealed` handler: PASS
- The choice is the caster's, and with only one card revealed it is taken
  automatically rather than presenting a choice of one: PASS
- The spell stays on the stack while the choice is pending, and the engine moves
  it once the chain completes (CR 608.2m): PASS
- Flashback {6}{B}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mill event for the discarded cards: `multi_target_and_mill.rs:forbidden_alchemy_emits_creature_card_milled_for_the_rest`

## Audit — 2026-08-28 18:29

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Forbidden Alchemy"`, https://scryfall.com/card/isd/55/forbidden-alchemy
**Oracle text**:
```
Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Instant
**Mana cost**: {2}{U}   **Keywords**: Flashback
**Rulings**: 7 — six generic flashback, and one specific: "If you have fewer than four cards in
your library, you'll look at all the cards there and put one into your hand and the rest into
your graveyard."
**Status**: ISSUE (a tutor, found here and fixed across the choice arms; plus one cleanup)

### Code issues

**1. Engine — the card kept was taken on trust.** Found on this card, fixed for every
resolution choice.

- Oracle text says: `Put **one of them** into your hand` — one of the four this spell looked at.
- Code did: the `ChooseFromRevealed` arm of `resolve_choice` went straight to
  `state.move_object(*keep_id, Zone::Hand, registry)` without asking whether `keep_id` was in
  `revealed`.

So answering the prompt with any card in your library pulled it to hand: a tutor, on an instant
meant to dig exactly four deep. Neither client picks a whole offered action — both assemble one
from per-slot choices — so this is reachable the same way the target holes were.

`ChooseTarget` was the arm the earlier audits kept arriving at and the only one that had been
checked. Its six siblings were all in the same state; the sweep is in the preceding commit, and
the second-worst was "that player discards a card" accepting a card from their library or
somebody else's hand.

**2. The card edited `library_order` by hand.**
- Code did: `let revealed: Vec<ObjectId> = player.library_order.drain(..count).collect();`
- Looking at cards moves nothing (CR 701.16a). Draining them left four cards whose zone said
  `Library` while the library did not list them — and here that state persisted for as long as
  the prompt stayed open, not just within one resolution. All four leave in the answer, so
  `move_object` takes each out of the order then. This is the same cleanup Mulch got; the
  library guard test now forbids `drain` outright, which it could not while this card needed it.

Card data is correct: `{2}{U}`, `CardType::Instant`, `flashback_cost: Some({6}{B})`, oracle text
verbatim, no target requirement (it looks at your own library and targets nothing).

### Tricky interactions checked
- **"One of them"**: fixed above.
- **The short-library ruling**: PASS. `take(4)` on a shorter library yields what is there; with
  exactly one card there is nothing to decide, so the card goes to hand without a prompt; with
  none, the spell resolves having looked at nothing.
- **"The rest into your graveyard" is a mill**: PASS — the rest go through `mill_one`, so a
  creature among them is visible to a watcher.
- **Instant timing**: PASS, and the flashback cast is an instant too.
- **Flashback {6}{B}** — a different colour from the card's own cost: PASS, and
  `flashback_multiple_instances.rs` is about exactly this ("only red mana available" against a
  granted black cost).
- **Cast via flashback, then exiled**: engine-side, tested generically.

### Test coverage
- one card to hand, the other three to the graveyard: `flashback.rs:437 forbidden_alchemy_draws_and_mills`
- a choice is offered from the top four: `cards_morbid_and_ltb.rs:1271`
- a creature among "the rest" is announced as milled: `multi_target_and_mill.rs:171`
- the short-library ruling, all three shapes:
  `flashback.rs:~480 forbidden_alchemy_looks_at_what_is_there_when_the_library_is_short` (NEW)
- a card that was not revealed cannot be kept:
  `submitted_targets.rs:~190 other_choices::a_card_that_was_not_revealed_cannot_be_the_one_you_keep` (NEW)
- and its sibling for the discard prompt: `submitted_targets.rs:~230` (NEW)

Mutation-checked: looking at three cards instead of four fails the main test; making the
one-card case raise a prompt fails the short-library test; disabling the `revealed` containment
check fails the tutor test and only it.

### Changes made
- `engine/actions/choices.rs`: containment checks on all six unvalidated choice arms.
- `forbidden_alchemy.rs`: reads the top four ids instead of draining them.
- `test_suite_guards.rs`: the library guard now forbids `drain` too.
- `flashback.rs`, `submitted_targets.rs`: three new tests.
