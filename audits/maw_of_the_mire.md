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
