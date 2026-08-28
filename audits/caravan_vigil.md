## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/173/caravan-vigil?utm_source=api
**Type line**: `Sorcery` — {G}
**Oracle text**:
```
Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Search your library for a basic land card ... put it into your hand, then
  shuffle" — a Basic supertype *and* the Land card type, so a nonbasic is not
  offered: PASS
- Every basic in the library is offered, not the first found: PASS
- "Morbid — **You may** put that card onto the battlefield **instead** ... if a
  creature died this turn" — the choice is offered only when the condition
  holds, and declining puts it in hand: PASS
- The morbid condition is checked at resolution: PASS
- Onto the battlefield untapped, and it does not count as a land drop: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The search, the morbid choice, and declining: `auto_pick.rs`, `cards_morbid_and_ltb.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/173/caravan-vigil?utm_source=api
**Type line**: `Sorcery` — {G}
**Oracle text**:
```
Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
```

**Rulings fetched**:
- [2011-09-22] You can choose to put the basic land card into your hand even if a creature died the turn you cast Caravan Vigil.

**Status**: ISSUE (fixed)

### Code issues

One found, and it turned out to be shared.

1. **The search took the basic land for the player.** `caravan_vigil.rs:82-100` (before the fix)
   - Oracle text says: `Search your library for a basic land card, reveal it, put it into your hand, then shuffle.`
   - Code did: `1 => Self::finish_search(state, object_id, basic_lands[0], controller, registry),` for the single-candidate case, and `optional: false` on the `ChooseTarget` for the multi-candidate case.
   - CR 701.19b: "If a player is searching a hidden zone for cards with stated characteristics ... that player isn't required to find some or all of those cards even if they're present in that zone." Mandatory means the player must search and must shuffle (CR 701.19a) — not that they must take a card. Declining matters here beyond flavour: the land goes to the graveyard's owner's library rather than revealing itself, and with morbid live the player may prefer to keep the land hidden rather than announce a basic.
   - Now the list is offered as `optional: true`, and declining still shuffles.

**Set-wide follow-up.** The shared `helpers::search_library` had the same bug in its mandatory path — auto-taking whenever exactly one card matched, and offering no decline when several did — so Traveler's Amulet and Garruk, the Veil-Cursed's tutor force-found too. Bitterheart Witch was the only card in the set that had this right, and it had opted out of the helper to get it. Fixed in the helper: `ChooseFromLibrary` now always offers "take none of them", and taking none shuffles. Five tests that drove a search through the old auto-take were updated to answer the prompt — they had been enshrining the force-find.

**Enabling change.** Declining an optional `ChooseTarget` previously did nothing at all, so a card could not tell a declined choice from a choice never offered. `CardBehavior::on_declined_choice` is the counterpart of `resolve_card_effect` for "none of them", with a no-op default; Caravan Vigil uses it to shuffle after a failed find.

**Also**: the "no matching card found in library" log was at `Event` level, which is shown to both players. A player who searches and comes back with nothing is not obliged to say whether there was anything to find; the line said it for them. Now `Debug`, in both the helper and this card.

### Checked against the ruling

- `You can choose to put the basic land card into your hand even if a creature died the turn you cast Caravan Vigil.` — PASS. Morbid is a `YesNo` whose "No" branch puts the land into hand, and the prompt says so: `"...put {land} onto the battlefield? (No = put into hand)"`.

### Checked and correct

- Cost `{G}`, `Sorcery`, oracle text verbatim.
- The morbid condition reads `state.creature_died_this_turn` at resolution. Morbid here is part of the effect, not an intervening-if — there is no trigger to gate — so resolution is the right moment.
- Basic land detection is `has_card_type(Land)` plus the `Basic` supertype from `face_data`, not a name match.
- Candidates come from `library_order`, which is a `Vec`, so the offered order is the library order and is stable.
- The shuffle happens on every path: found and put in hand, found and put onto the battlefield via morbid, found nothing, and nothing to find.
- The card does not move its own spell off the stack.

### Noted, not fixed

`finish_search` removes the land from `library_order` before the morbid question is asked, so while that question is pending the land's `zone` is still `Library` but it is no longer in the library's order. Nothing in the set reads library contents during another player's pending choice, and the land is moved for real as soon as the question is answered — but the two representations disagree in that window. Reconciling them means making `move_object` the only thing that maintains `library_order`, which is a state-model change rather than a card fix.

### Tricky interactions checked

- One basic land in the library: offered, not taken. PASS (after fix).
- Declining the find: land stays, library still shuffled. PASS (after fix).
- No basic land at all: shuffles, and does not announce that the library held none. PASS (after fix).
- Morbid live, player picks hand: PASS.
- Morbid live, player picks battlefield: PASS.
- No creature died: no morbid question at all. PASS.

### Test coverage

- morbid offered only if a creature died, and yes puts the land onto the battlefield: `cards_graveyard_interaction.rs:114`
- may search and find nothing, and shuffle anyway: `cards_graveyard_interaction.rs` `caravan_vigil_may_search_and_find_nothing` (NEW, mutation-checked)
- the same rule through the shared helper: `cards_equipment_and_artifacts.rs` `a_mandatory_library_search_may_still_find_nothing` (NEW, mutation-checked)
- declining the find with morbid live asks no morbid question: covered by the new fail-to-find test's `awaiting_action.is_none()` assertion.

