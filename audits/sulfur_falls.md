# Audit: Sulfur Falls

## Oracle (Scryfall)
- **Name:** Sulfur Falls
- **Cost:** N/A (Land)
- **Type:** Land
- **Oracle:** Sulfur Falls enters the battlefield tapped unless you control an Island or a Mountain. {T}: Add {U} or {R}.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/sulfur_falls.rs`
- **Name:** Sulfur Falls ✅
- **Cost:** None ✅
- **Type:** Land ✅
- **ETB tapped condition:** checks for Island or Mountain subtypes among other lands (excludes self) ✅
- **Mana abilities:** Add {U} or Add {R} ✅
- **requires_tap:** true for both abilities ✅
- **Zone check:** only when on battlefield and untapped ✅

## Verdict: PASS -- no issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: This land enters tapped unless you control an Island or a Mountain.\n{T}: Add {U} or {R}.
**Type line**: Land
**Mana Cost**: None
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch**: Oracle says `"This land enters tapped unless you control an Island or a Mountain."` but code has `"Sulfur Falls enters the battlefield tapped unless you control an Island or a Mountain."`. Modern oracle template uses "This land enters tapped" not "enters the battlefield tapped".
### Behavior
Correct. on_enter_battlefield checks for Island or Mountain subtypes among controller's other permanents and taps if none found. Mana abilities correctly produce {U} or {R} with tap cost.
