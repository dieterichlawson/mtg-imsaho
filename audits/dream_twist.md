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
