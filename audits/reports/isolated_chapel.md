# Audit: Isolated Chapel

## Oracle (Official)
- **Name:** Isolated Chapel
- **Cost:** (none — Land)
- **Type:** Land
- **Oracle:** Isolated Chapel enters the battlefield tapped unless you control a Plains or a Swamp. {T}: Add {W} or {B}.
- **P/T:** N/A

## Implementation
- Name: "Isolated Chapel" -- CORRECT
- Cost: None -- CORRECT
- Type: Land -- CORRECT
- Oracle text matches -- CORRECT
- ETB tapped-unless logic checks for Plains or Swamp subtypes, excluding self -- CORRECT
- Mana abilities produce {W} or {B} with tap -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Isolated Chapel
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Land
- **Oracle:** This land enters tapped unless you control a Plains or a Swamp. / {T}: Add {W} or {B}.

### Card Data
- **Name:** Isolated Chapel -- PASS
- **Cost:** None -- PASS
- **Types:** Land -- PASS
- **Subtypes:** (none) -- PASS
- **P/T:** None -- PASS

### Oracle Text Match
- Code oracle_text uses old-style "Isolated Chapel enters the battlefield tapped" vs current oracle "This land enters tapped". Cosmetic only.
- PASS (minor wording variance, no functional impact)

### Behavior Audit
- **ETB tapped condition:** Checks controller's other permanents for Plains or Swamp subtypes, excluding self. Correctly enters tapped if no match. -- PASS
- **Mana abilities:** Produces {W} or {B}, requires tap, only on battlefield when untapped. -- PASS

### Result: PASS

## Audit — 2026-04-03 07:08
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/242/isolated-chapel?utm_source=api, cached 2026-04-01)
**Oracle text**: This land enters tapped unless you control a Plains or a Swamp. / {T}: Add {W} or {B}.
**Type line**: Land

**Status**: ISSUE

### Code issues
1. **Oracle text string mismatch (cosmetic):** The `oracle_text` field in `card_data()` reads `"Isolated Chapel enters the battlefield tapped unless you control a Plains or a Swamp."` but current Scryfall oracle text is `"This land enters tapped unless you control a Plains or a Swamp."`. The sibling checkland Clifftop Retreat already uses the updated wording. The doc comment on line 7 has the same stale wording. No functional impact, but the stored oracle text does not match the authoritative source.

### Tricky interactions checked (min 3)
1. **Self-exclusion on ETB:** `controller_has_matching_land` correctly excludes `object_id` from the check (`o.id != object_id`), so Isolated Chapel does not count itself (it has no land subtypes anyway, but this is good hygiene and matters if the card were somehow given a Plains/Swamp subtype).
2. **Dual lands / Shocklands as enablers:** The check uses `o.subtypes.iter().any(|s| s == "Plains") || o.subtypes.iter().any(|s| s == "Swamp")`, which correctly detects any permanent with the Plains or Swamp subtype (e.g., Godless Shrine, Scrubland), not just basic lands. This is correct behavior.
3. **Mana ability gating:** `mana_abilities()` returns empty if the object is not on the battlefield or is already tapped, preventing illegal activations. The `requires_tap: true` flag ensures the engine taps the land on activation.

### Test coverage
- **Existing:** One test (`isolated_chapel_card_data` in `innistrad_simple_cards.rs`) verifies the card type is Land. Minimal.
- **Missing:** No tests for ETB tapped/untapped condition (with and without Plains/Swamp on battlefield), no tests for mana production, no tests for dual-land-subtype enablers.
