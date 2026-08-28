## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/196/mulch?utm_source=api
**Type line**: `Sorcery` — {1}{G}
**Oracle text**:
```
Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
```
**Status**: PASS

### Code issues
No issues found.

Reveals the top four, lands to hand and the rest to the graveyard, using `has_card_type(Land)`. Handles a library of fewer than four.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/196/mulch?utm_source=api
**Type line**: `Sorcery` — {1}{G}
**Oracle text**:
```
Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Put **all land cards** revealed this way into your hand and **the rest** into
  your graveyard" — lands to hand, everything else to the graveyard, with no
  choice: PASS
- The graveyard half is a library-to-graveyard move, so it goes through
  `mill_one` and a creature card among them emits `CreatureCardMilled`: PASS
- A library with fewer than four cards reveals what it has: PASS
- The land test reads the card's active face rather than the object's empty
  `card_types`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The split and the mill event: `multi_target_and_mill.rs:mulch_emits_creature_card_milled`

## Audit — 2026-08-28 17:39

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Mulch"`, https://scryfall.com/card/isd/196/mulch
**Oracle text**:
```
Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
```
**Type line**: Sorcery
**Mana cost**: {1}{G}
**Rulings**: none on Scryfall for this card.
**Status**: ISSUE (one cleanup; behaviour was already correct)

### Code issues

**The card edited `library_order` by hand.**
- Code did: `let revealed: Vec<ObjectId> = player.library_order.drain(..count).collect();`
- That took four cards out of the library's order while their `zone` still said `Library`, so
  for the rest of the resolution the two halves of the library disagreed. Nothing observes it
  in this card's case — every one of the four leaves the library before the resolution ends —
  but nothing needs it either: `move_object` takes a card out of the order as it leaves
  (CR 401.1), which is why the guard test `only_the_library_helper_puts_a_card_into_a_library`
  exists. Now it reads the top four ids and lets each move do its own bookkeeping.

The guard's `drain` carve-out named both Mulch and Forbidden Alchemy. Forbidden Alchemy keeps
it — its "put one into your hand" is a choice prompt, so its revealed cards genuinely sit
outside the order across a priority window. That is its own audit's question; the comment now
says which card it is for.

Everything else is right: `{1}{G}`, `CardType::Sorcery`, oracle text verbatim, no target
requirement ("reveal the top four" targets nothing), `min(4, len)` for a short library,
`has_card_type(.., Land, ..)` reading the active face, and the non-lands routed through
`mill_one` so a creature among them is visible to a watcher.

### Tricky interactions checked
- **"the top four"**: PASS — the fifth card is not revealed and the remaining order is intact.
- **A library with fewer than four cards**: PASS. All of them are revealed, and revealing is not
  drawing, so nobody loses (CR 704.5b).
- **A card that is both a land and something else**: `has_card_type` is a membership test, not
  an equality one, so a land creature would go to hand. Nothing in this pool is both.
- **A creature card among the non-lands is a library-to-graveyard move**: PASS —
  `CreatureCardMilled` comes from `move_object`, so Undead Alchemist sees it.
- **Sorcery timing**: engine-side.
- **Mulch given flashback by Past in Flames**: covered in `flashback.rs`.

### Test coverage
- two lands and two non-lands go to their right zones:
  `cards_graveyard_interaction.rs:298 mulch_puts_lands_in_hand_and_rest_in_graveyard`
- only the top four, and the rest of the library keeps its order:
  `cards_graveyard_interaction.rs:328 mulch_reveals_only_the_top_four` (NEW)
- a short library reveals what is there and is not a loss:
  `cards_graveyard_interaction.rs:358 mulch_with_a_short_library_reveals_what_is_there` (NEW)
- a milled creature card is announced: `multi_target_and_mill.rs:106 mulch_emits_creature_card_milled`
- castable from the graveyard on a granted flashback: `flashback.rs:850`, `flashback.rs:933`

Mutation-checked: revealing the whole library instead of four kills the new top-four test;
treating nothing as a land kills all three.

The original test stocked exactly four cards, so it could not tell "the top four" from "the
whole library" — the first mutation passed against it.

### Changes made
- `mulch.rs`: reads the top four ids instead of draining them out of `library_order`.
- `cards_graveyard_interaction.rs`: two new tests.
- `test_suite_guards.rs`: the `drain` carve-out comment now names only Forbidden Alchemy, and
  says why it still needs it.
