# Audit: Fortress Crab

## Reference (Scryfall)
- **Name:** Fortress Crab
- **Cost:** {3}{U}
- **Type:** Creature -- Crab
- **Oracle:** (none -- vanilla creature)
- **P/T:** 1/6

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{U})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Crab)
- Oracle text: CORRECT (empty string)
- P/T: CORRECT (1/6)

## Issues
None found.

## Audit 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
- **Name:** Fortress Crab
- **Mana Cost:** {3}{U}
- **Type Line:** Creature — Crab
- **P/T:** 1/6
- **Oracle Text:** (none — vanilla creature)

### Implementation Audit (`mtg-engine/src/cards/isd/fortress_crab.rs`)
- Name: PASS — "Fortress Crab"
- Mana Cost: PASS — Generic(3) + Blue = {3}{U}
- Card Type: PASS — Creature
- Supertypes: PASS — none
- Subtypes: PASS — "Crab"
- Power/Toughness: PASS — 1/6
- Oracle Text: PASS — empty string (vanilla)
- Keywords: PASS — none

### Test Coverage
- `fortress_crab_is_1_6` in `mtg-engine/tests/innistrad_cards.rs`: verifies P/T = 1/6. PASSES.

### Result
**ALL PASS** — no mismatches found.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: (no oracle text — vanilla creature)
**Type line**: Creature — Crab
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:58
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/56/fortress-crab?utm_source=api), cached 2026-04-01
**Oracle text**: *(no text — vanilla creature)*
**Type line**: Creature — Crab
**Status**: PASS

### Code issues
None. Implementation is clean and follows the same pattern as other vanilla creatures in the set.

### Tricky interactions checked (min 3)
1. **No abilities to misimplement**: Confirmed `oracle_text` is `String::new()`, `keywords` is empty vec, no triggered/activated abilities defined. Matches vanilla status from Scryfall.
2. **Mana cost correctness**: `{3}{U}` correctly represented as `Generic(3) + Colored(Color::Blue)`. Order is correct (generic first).
3. **Creature subtype**: Single subtype "Crab" matches Scryfall type line "Creature — Crab". No supertypes, which is correct.
4. **P/T values**: 1/6 matches oracle. High toughness, low power — confirmed not transposed.

### Test coverage
- `fortress_crab_is_1_6` in `mtg-engine/tests/innistrad_cards.rs`: verifies power=1 and toughness=6. Test passes.
- Card is registered in card registry (`mtg-engine/src/cards/mod.rs`) and included in the `innistrad-blue` decklist.
- No behavioral tests needed (vanilla creature with no abilities).
