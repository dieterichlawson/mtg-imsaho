# Audit: Make a Wish

## Oracle (Official)
- **Name:** Make a Wish
- **Cost:** {3}{G}
- **Type:** Sorcery
- **Oracle:** Return two cards at random from your graveyard to your hand.
- **P/T:** N/A

## Implementation
- Name: "Make a Wish" -- CORRECT
- Cost: {3}{G} -- CORRECT
- Type: Sorcery -- CORRECT
- Oracle text matches -- CORRECT
- Shuffles graveyard cards and takes 2 at random -- CORRECT
- Excludes tokens from selection -- CORRECT
- Excludes self (the Make a Wish spell) from graveyard selection -- CORRECT
- Calls move_spell_after_resolve -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Make a Wish
- **Cost:** {3}{G}
- **Type:** Sorcery
- **Oracle Text:** Return two cards at random from your graveyard to your hand.

### Card Data Checks
- [x] Name: "Make a Wish" — correct
- [x] Cost: {3}{G} — correct
- [x] Types: Sorcery — correct
- [x] Oracle text matches — correct

### Behavior Checks
- [x] Gets cards from controller's graveyard — correct
- [x] Excludes self (Make a Wish itself) from candidates — correct
- [x] Excludes tokens — correct
- [x] Shuffles and takes up to 2 at random — correct
- [x] Returns 1 card if only 1 available (per ruling) — correct
- [x] Handles empty graveyard gracefully — correct
- [x] Spell moves to graveyard after resolve — correct

### Result: PASS

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/192/make-a-wish)
**Oracle text**: "Return two cards at random from your graveyard to your hand."
**Type line**: Sorcery
**Status**: PASS

### Code issues
None found.

- **Name**: "Make a Wish" -- matches oracle.
- **Mana cost**: `Generic(3), Colored(Green)` = {3}{G} -- matches oracle.
- **Card types**: `Sorcery` -- matches oracle type line.
- **Oracle text field**: `"Return two cards at random from your graveyard to your hand."` -- matches oracle exactly.
- **on_resolve logic**:
  - Retrieves controller's graveyard via `objects_in_zone(Zone::Graveyard, controller)`. The engine filters graveyards by `owner`, and for a spell on the stack `controller == owner`, so this is correct.
  - Filters out tokens (`!o.is_token`) -- defensive and correct (tokens cease to exist in graveyard via SBAs).
  - Filters out self (`o.id != object_id`) -- correct per ruling: Make a Wish is still on the stack during resolution.
  - Shuffles the list and takes up to 2 via `gy_cards.shuffle(&mut rng)` then `.take(2)` -- correct random selection.
  - If 0 or 1 cards in graveyard, `take(2)` returns 0 or 1 respectively -- matches ruling: "If you only have one card in your graveyard when Make a Wish resolves, that card will be returned to your hand."
  - Moves selected cards to Hand via `state.move_object(card_id, Zone::Hand)`.
  - Calls `move_spell_after_resolve(object_id)` at the end, which moves the spell to graveyard (or exile if cast with flashback) -- correct ordering (selection happens before self enters graveyard).
- **No targets required**: Correct -- random selection is not targeting.

### Tricky interactions checked (min 3)
1. **Self-exclusion during resolution**: Make a Wish filters `o.id != object_id` to exclude itself from the graveyard candidates. Since the spell is on the stack (not in graveyard) during resolution, this filter is redundant but harmless. The spell only enters the graveyard after `move_spell_after_resolve` is called at the end. Correct behavior per ruling.
2. **Fewer than 2 cards in graveyard**: `take(2)` on a vec with 0 or 1 elements returns 0 or 1 elements respectively. The empty case logs "no cards in graveyard to return". Matches ruling: "If you only have one card in your graveyard when Make a Wish resolves, that card will be returned to your hand."
3. **Flashback interaction**: If cast via flashback, `move_spell_after_resolve` exiles the spell instead of putting it in the graveyard. This does not affect the random selection since the selection happens before the spell leaves the stack.
4. **Token exclusion**: Tokens that die go to the graveyard momentarily before SBAs remove them. The `!o.is_token` filter prevents selecting any lingering tokens. Correct defensive coding.

### Test coverage
- `make_a_wish_card_data`: Verifies card type is Sorcery and mana value is 4.
- `make_a_wish_returns_cards_from_graveyard`: Puts 3 creatures in graveyard, casts Make a Wish, verifies exactly 2 are returned to hand.
- **Missing**: No test for empty graveyard case, no test for exactly 1 card in graveyard. These are minor gaps -- the logic is trivially correct via `take(2)` on a shorter vec.
