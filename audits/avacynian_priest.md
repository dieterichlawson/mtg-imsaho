# Audit: Avacynian Priest

## Oracle (Scryfall)
- **Name:** Avacynian Priest
- **Cost:** {1}{W}
- **Type:** Creature — Human Cleric
- **Oracle:** {1}, {T}: Tap target non-Human creature.
- **P/T:** 1/2

## Implementation: `mtg-engine/src/cards/avacynian_priest.rs`
- **Name:** Avacynian Priest ✅
- **Cost:** {1}{W} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Cleric ✅
- **P/T:** 1/2 ✅
- **Oracle text:** matches ✅
- **Activated ability:** {1}, {T}: Tap target non-Human creature ✅
- **Target filtering:** Excludes Humans via `is_valid_target` ✅
- **Tap effect:** Sets `tapped = true` on target ✅
- **requires_tap:** true ✅
- **Triggered abilities:** none ✅

## Verdict: PASS — no issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {1}, {T}: Tap target non-Human creature.
**Type line**: Creature — Human Cleric
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Avacynian Priest", cost {1}{W}, 1/2, type Creature — Human Cleric. Activated ability costs {1} + tap, targets non-Human creatures on the battlefield. is_valid_target correctly excludes Humans. on_activate_ability sets target tapped. All correct.
