# Audit: Desperate Ravings

## Reference (Scryfall)
- **Name:** Desperate Ravings
- **Cost:** {1}{R}
- **Type:** Instant
- **Oracle:** Draw two cards, then discard a card at random. Flashback {2}{U}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{R})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Flashback cost: CORRECT ({2}{U})
- P/T: CORRECT (N/A)
- on_resolve draws 2 cards: CORRECT
- on_resolve discards at random: CORRECT (uses `choose(&mut rand::thread_rng())`)

## Issues
None found.

---

## Audit 2 (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
```
Draw two cards, then discard a card at random.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

### Implementation File
`mtg-engine/src/cards/isd/desperate_ravings.rs`

### Card Data
- **Name:** CORRECT — `"Desperate Ravings"`
- **Mana cost:** CORRECT — `{1}{R}` via `Generic(1), Colored(Red)`
- **Type:** CORRECT — `Instant`
- **Oracle text field:** CORRECT — `"Draw two cards, then discard a card at random."`
- **Flashback cost:** CORRECT — `{2}{U}` via `Generic(2), Colored(Blue)`
- **P/T:** CORRECT — `None`/`None`
- **Keywords:** `vec![]` — consistent with all other flashback cards in the codebase, which represent flashback via the `flashback_cost` field rather than the keywords list.

### on_resolve Logic
1. **Draw 2 cards:** CORRECT — calls `crate::engine::draw_cards(state, controller, 2)`.
2. **Discard a card at random:** CORRECT — collects all hand objects for the controller, uses `choose(&mut rand::thread_rng())` to pick one at random, moves it to graveyard, and emits a `Discarded` event.
3. **move_spell_after_resolve:** CORRECT — called at the end; delegates to the shared helper that checks `cast_with_flashback` to decide between exile (flashback) and graveyard (normal cast).

### Potential Concern (not a bug)
The hand filter uses `o.owner == controller` rather than `o.controller == controller`. For cards in hand, owner and controller are always equal, and this pattern is used consistently throughout the codebase. Not a functional issue.

### Test Coverage
- `mtg-engine/tests/flashback.rs` — `desperate_ravings_draws_two_discards_one`: verifies net hand size is +1 after casting (draw 2, discard 1, minus the spell itself). No flashback-specific test for this card, but flashback behavior is shared infrastructure tested elsewhere.

### Issues
None found. Implementation matches oracle text.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Draw two cards, then discard a card at random. / Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.
