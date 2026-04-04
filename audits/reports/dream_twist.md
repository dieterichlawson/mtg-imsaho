# Audit: Dream Twist

## Reference (Scryfall)
- **Name:** Dream Twist
- **Cost:** {U}
- **Type:** Instant
- **Oracle:** Target player mills three cards. Flashback {1}{U}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({U})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Flashback cost: CORRECT ({1}{U})
- Target requirement: CORRECT (PlayerOnly)
- Mills 3 cards: CORRECT
- P/T: CORRECT (N/A)

## Issues
None found.

## Audit (2026-04-02)

### Oracle Text (Scryfall)
> Target player mills three cards.
> Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)

### Implementation Summary (`mtg-engine/src/cards/isd/dream_twist.rs`)
- **Name:** "Dream Twist"
- **Cost:** {U} (`ManaSymbol::Colored(Color::Blue)`)
- **Type:** Instant
- **oracle_text field:** "Target player mills three cards."
- **flashback_cost:** {1}{U} (`Generic(1), Colored(Color::Blue)`)
- **target_requirement:** `PlayerOnly`
- **on_resolve:** calls `mill_cards(state, *player_id, 3)`, then `move_spell_after_resolve(object_id)`

### Checklist
| Check                  | Result  | Notes                                                        |
|------------------------|---------|--------------------------------------------------------------|
| Mana cost              | CORRECT | {U}                                                          |
| Card type              | CORRECT | Instant                                                      |
| Targeting              | CORRECT | PlayerOnly — matches "Target player"                         |
| Mill amount            | CORRECT | 3 — matches "mills three cards"                              |
| Flashback cost         | CORRECT | {1}{U}                                                       |
| move_spell_after_resolve | CORRECT | Called after mill effect                                    |
| Keywords vec           | OK      | Empty; flashback handled via flashback_cost field, mill via engine function |

### Tests
- `dream_twist_mills_three` (`mtg-engine/tests/flashback.rs:229`): Stocks P1 library with 5 cards, casts Dream Twist targeting P1, asserts 3 cards milled to graveyard and 2 remain in library.
- Flashback exile behavior covered generically by `flashback_spell_is_exiled_after_resolve` and `flashback_spell_countered_is_exiled` tests (using Geistflame).

### Verdict
**No mismatches found.** Implementation faithfully matches oracle text.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Target player mills three cards. / Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:54
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/54/dream-twist)
**Oracle text**: Target player mills three cards.\nFlashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
None. Implementation correctly matches oracle text in all respects:
- Mana cost {U} is correct.
- Card type Instant is correct.
- Target requirement `PlayerOnly` correctly implements "Target player".
- `mill_cards(state, *player_id, 3)` correctly mills exactly 3 cards.
- `flashback_cost` of {1}{U} (Generic(1), Colored(Blue)) is correct.
- `move_spell_after_resolve` correctly handles flashback exile vs. normal graveyard routing.
- `mill_cards` engine function gracefully handles libraries with fewer than 3 cards (mills as many as available), matching MTG rules.

### Tricky interactions checked (min 3)
1. **Hexproof player targeting**: Verified via `witchbane_orb.rs::can_target_self_with_hexproof` -- Dream Twist can target self even with hexproof (hexproof only prevents opponents from targeting). Opponent targeting a hexproof player is blocked by `opponent_cannot_target_hexproof_player`.
2. **Flashback exile**: Verified via `flashback.rs::flashback_spell_is_exiled_after_resolve` and `flashback_spell_countered_is_exiled` -- spells cast via flashback are exiled whether they resolve or are countered, via `move_spell_after_resolve` checking `cast_with_flashback`.
3. **Milling with near-empty library**: The `mill_cards` function (engine.rs:2755) breaks early when `library_order.is_empty()`, so milling 3 from a library with 1-2 cards mills only what is available without panic or incorrect behavior.

### Test coverage
- `flashback.rs::dream_twist_mills_three` (line 229): Casts Dream Twist from hand targeting P1 with 5-card library, asserts 3 milled to graveyard and 2 remain.
- `flashback.rs::mill_cards_moves_to_graveyard` (line 166): Unit test for the `mill_cards` engine function.
- `witchbane_orb.rs::can_target_self_with_hexproof` (line 62): Uses Dream Twist to verify self-targeting with hexproof.
- Generic flashback tests (`flashback_offered_from_graveyard`, `flashback_spell_is_exiled_after_resolve`, `flashback_spell_countered_is_exiled`) cover flashback mechanics used by Dream Twist.
