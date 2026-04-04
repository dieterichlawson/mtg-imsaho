# Audit: Maw of the Mire

## Reference (Scryfall/API)
- **Name:** Maw of the Mire
- **Mana Cost:** {4}{B}
- **Type:** Sorcery
- **Oracle:** Destroy target land. You gain 4 life.

## Implementation: `maw_of_the_mire.rs`
- **Name:** Maw of the Mire -- CORRECT
- **Mana Cost:** {4}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Target:** Land (PermanentWithFilter HasCardType Land) -- CORRECT
- **Destroy target land:** Uses `try_destroy` after checking battlefield -- CORRECT
- **Life gain:** Gains 4 life for controller -- CORRECT

### ISSUE: Life gain occurs even when target land is no longer on the battlefield

The code gains 4 life unconditionally (lines 63-73) even if the target land was not on the battlefield when the spell resolves (the if-block on line 55 only guards the destroy, not the life gain). Per the ruling: "If the targeted land is an illegal target by the time Maw of the Mire resolves, it won't resolve and none of its effects will occur. You won't gain 4 life." If the engine does not pre-check target legality before calling `on_resolve`, the life gain fires incorrectly.

- **Code (line 55+63):** Destroy is gated on battlefield check, but life gain on line 63 is unconditional
- **Oracle ruling:** "If the targeted land is an illegal target by the time Maw of the Mire resolves, it won't resolve and none of its effects will occur."

## Verdict: ISSUE -- Life gain not gated on successful target resolution

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Destroy target land. You gain 4 life.
**Type line**: Sorcery
**Status**: ISSUE
### Code issues
Life gain (lines 63-73) is outside the target-validity if-block (line 55). If the target land left the battlefield before resolution but `on_resolve` is still called, the spell would gain 4 life without destroying anything. The life gain should be inside the same block as the destroy, or the entire method should early-return if the target is invalid.

## Re-audit — 2026-04-02
**Status**: PASS
Previously fixed bug re-verified: on_resolve correctly checks target validity, destroys land, then gains 4 life only if target was valid. Oracle text matches Scryfall verbatim. Behavior unchanged.

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/108/maw-of-the-mire)
**Oracle text**: Destroy target land. You gain 4 life.
**Type line**: Sorcery
**Status**: PASS
### Code issues
None. All card data matches oracle text. Mana cost {4}{B} (Generic(4) + Black) is correct (CMC 5). Target requirement `PermanentWithFilter(HasCardType(Land))` correctly restricts to lands on the battlefield. `is_valid_target` validates zone and card type. `on_resolve` checks target is still on the battlefield (lines 56-59, early return if not), calls `try_destroy` for the land, then gains 4 life -- all inside the target validity guard. Life gain correctly uses `LifeChanged` event. Spell cleanup uses `move_spell_after_resolve`. The engine also fizzles the spell before calling `on_resolve` if all targets are illegal (stack.rs CR 608.2b check), providing double coverage.
### Tricky interactions checked (min 3)
1. **Indestructible land**: `try_destroy` returns `Indestructible` without destroying it. The code ignores the return value and proceeds to gain 4 life. This is correct -- the spell still resolves and the life gain is not contingent on the destruction succeeding.
2. **Target land leaves battlefield before resolution**: The engine's fizzle check (stack.rs lines 74-87) prevents `on_resolve` from being called. Additionally, the card's own zone check (line 56) provides a defensive second layer. No life is gained. Matches the Scryfall ruling.
3. **Target land is bounced and a different land enters with same name**: The target tracks by ObjectId, not name. The original ObjectId is no longer on the battlefield, so the spell fizzles correctly.
### Test coverage
- `maw_of_the_mire_card_data`: Verifies card type (Sorcery) and mana value (5). PASS.
- `maw_of_the_mire_destroys_land_and_gains_life`: Casts against opponent's Forest, verifies land goes to graveyard and caster gains 4 life (20 -> 24). PASS.
- Missing: no test for fizzle case (target removed before resolution), no test for indestructible land interaction. These are low priority since the engine fizzle logic and `try_destroy` are tested elsewhere.
