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

## Audit — 2026-04-02 20:50
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Draw two cards, then discard a card at random.\nFlashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found. Implementation correctly:
- Sets mana cost to {1}{R}, type Instant, flashback cost {2}{U}
- Draws 2 cards via `engine::draw_cards`
- Discards 1 card at random via `rand::thread_rng().choose()`
- Emits `Discarded` event for triggers (e.g. Burning Vengeance, Murder of Crows)
- Delegates post-resolution zone movement to `move_spell_after_resolve` (exile if flashback, graveyard otherwise)

### Tricky interactions checked (min 3)
1. **Flashback + exile**: When cast from graveyard via flashback, `move_spell_after_resolve` checks `cast_with_flashback` and exiles the card. Verified via shared flashback infrastructure tests (`flashback_spell_is_exiled_after_resolve`, `flashback_spell_countered_is_exiled`).
2. **Empty hand after draw**: If controller starts with 0 cards in hand (spell is on the stack), draws 2, then `choose` on a 2-card hand always returns `Some` — correctly discards 1 at random, leaving 1 card.
3. **Drawing from empty library**: If library has fewer than 2 cards, `draw_cards` handles the state-based action (player loses for drawing from empty library). The random discard still executes if the player has cards, and the `if let Some` guard handles the 0-card case.
4. **Burning Vengeance trigger**: Casting Desperate Ravings via flashback triggers Burning Vengeance, which checks `cast_from_gy` on resolved spells. Works correctly with the shared flashback infrastructure.

### Test coverage
- `mtg-engine/tests/flashback.rs::desperate_ravings_draws_two_discards_one` — verifies net hand size is +1 after casting (draw 2, discard 1, minus the spell itself)
- Flashback mechanics tested extensively via shared tests (offered from graveyard, not offered without mana, exiled after resolve, exiled when countered)
