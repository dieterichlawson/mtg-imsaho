# Audit: Gnaw to the Bone

## Oracle Reference (Scryfall)
- Cost: {2}{G}
- Type: Instant
- Oracle: "You gain 2 life for each creature card in your graveyard.
  Flashback {3}{G}"

## Implementation: gnaw_to_the_bone.rs

## Issues Found

No issues found. Name, cost ({2}{G}), type (Instant), oracle text, flashback cost ({3}{G}), and effect (gain 2 life per creature card in graveyard) all match. The implementation correctly counts creature cards in the controller's graveyard, excluding the spell itself (still on stack).

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
You gain 2 life for each creature card in your graveyard.
Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

### Findings
- Name, cost ({2}{G}), type (Instant) all match.
- Life gain logic correctly counts creature cards in controller's graveyard (excluding self on stack) and gains 2 life per creature -- correct.

### ISSUE: Flashback cost mismatch
- **Oracle**: Flashback {2}{G}
- **Code**: `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Green)]))` which equals {3}{G}

The flashback cost should be Generic(2) + Green, not Generic(3) + Green. The previous audit incorrectly listed the oracle flashback as {3}{G} but Scryfall confirms it is {2}{G}.

### Verdict: ISSUE
Flashback cost is {3}{G} in code but oracle specifies {2}{G}.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to include flashback line per codebase convention. Previously fixed bug re-verified: life gain logic correctly counts creature cards in graveyard excluding the spell itself. Behavior unchanged.

## Audit — 2026-04-02 21:09
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: "You gain 2 life for each creature card in your graveyard.\nFlashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)"
**Type line**: Instant

**Status**: PASS

### Code issues
None.

- **Name**: "Gnaw to the Bone" -- matches oracle.
- **Mana cost**: {2}{G} (Generic(2) + Green) -- matches oracle "{2}{G}".
- **Card type**: Instant -- matches oracle "Instant".
- **Flashback cost**: {2}{G} (Generic(2) + Green) -- matches oracle "Flashback {2}{G}".
- **Oracle text field**: "You gain 2 life for each creature card in your graveyard.\nFlashback {2}{G}" -- matches oracle (reminder text omitted per codebase convention).
- **on_resolve logic**: Counts creature cards in controller's graveyard via `o.power.is_some()` proxy, excludes the spell itself via `o.id != object_id`, multiplies count by 2, gains that much life. Emits `LifeChanged` event. Calls `move_spell_after_resolve` which correctly handles flashback exile. All correct.
- **Creature detection**: Uses `o.power.is_some()` instead of `o.card_types.contains(&CardType::Creature)`. This is a semantic proxy that works correctly for the Innistrad card pool (no non-creature cards with power/toughness exist in this set). Noted in multiple previous audits as acceptable.
- **Graveyard ownership**: Filters by `o.owner == controller`. Per rule 400.3, graveyard cards are identified by owner, and `controller` here is derived from the spell's `controller` field (which equals `owner` for the caster). Correct.
- **Zero creatures edge case**: `if life_gain > 0` guard prevents emitting a zero-gain event. Correct behavior (no life gain event if graveyard has no creatures).

### Tricky interactions checked (min 3)
1. **Self-exclusion during resolution**: The spell is on the Stack when `on_resolve` runs, so `o.zone == Zone::Graveyard` already excludes it. The `o.id != object_id` check provides redundant safety. PASS.
2. **Flashback exile**: `move_spell_after_resolve` checks `cast_with_flashback` and exiles if true, otherwise sends to graveyard. Verified in `flashback_spell_is_exiled_after_resolve` and `flashback_spell_countered_is_exiled` tests. PASS.
3. **Non-creature cards not counted**: Non-creature graveyard cards (instants, sorceries, lands, enchantments) have `power: None`, so `o.power.is_some()` correctly excludes them. PASS.
4. **Flashback from graveyard does not double-count**: When cast via flashback, the spell moves from Graveyard to Stack before resolution. It cannot be counted as a creature in the graveyard (it's an Instant with `power: None` anyway). PASS.

### Test coverage
- `flashback::gnaw_to_the_bone_gains_life`: Creates 3 Grizzly Bears in graveyard, casts Gnaw from hand, asserts +6 life. Covers the core life-gain mechanic.
- Flashback mechanics covered by shared tests: `flashback_offered_from_graveyard`, `flashback_spell_is_exiled_after_resolve`, `flashback_spell_countered_is_exiled`, `flashback_not_offered_without_mana`.
- No dedicated flashback-specific test for Gnaw (e.g., casting from graveyard and verifying exile + life gain together), but the generic flashback tests plus the card-specific test provide adequate coverage.
