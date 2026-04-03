# Audit: Hinterland Harbor

## Oracle (Official)
- **Name:** Hinterland Harbor
- **Cost:** (none — Land)
- **Type:** Land
- **Oracle:** Hinterland Harbor enters the battlefield tapped unless you control a Forest or an Island. {T}: Add {G} or {U}.
- **P/T:** N/A

## Implementation
- Name: "Hinterland Harbor" -- CORRECT
- Cost: None -- CORRECT
- Type: Land -- CORRECT
- Oracle text matches -- CORRECT
- ETB tapped-unless logic checks for Forest or Island subtypes, excluding self -- CORRECT
- Mana abilities produce {G} or {U} with tap -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Hinterland Harbor
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Land
- **Oracle:** This land enters tapped unless you control a Forest or an Island. / {T}: Add {G} or {U}.

### Card Data
- **Name:** Hinterland Harbor -- PASS
- **Cost:** None -- PASS
- **Types:** Land -- PASS
- **Subtypes:** (none) -- PASS
- **P/T:** None -- PASS

### Oracle Text Match
- Code oracle_text uses old-style wording "Hinterland Harbor enters the battlefield tapped" vs current oracle "This land enters tapped". This is a cosmetic wording difference; the functional meaning is identical.
- PASS (minor wording variance, no functional impact)

### Behavior Audit
- **ETB tapped condition:** Checks controller's other permanents for Forest or Island subtypes, excluding self. Correctly enters tapped if no match. -- PASS
- **Mana abilities:** Produces {G} or {U}, requires tap, only on battlefield when untapped. -- PASS

### Result: PASS

## Audit — 2026-04-03 07:04
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/241/hinterland-harbor)
**Oracle text**: This land enters tapped unless you control a Forest or an Island.
{T}: Add {G} or {U}.
**Type line**: Land
**Status**: ISSUE

### Code issues
1. **Oracle text mismatch (line 37)**: Code stores `"Hinterland Harbor enters the battlefield tapped unless you control a Forest or an Island."` but current Scryfall oracle text is `"This land enters tapped unless you control a Forest or an Island."`. All four sibling checklands (Clifftop Retreat, Isolated Chapel, Sulfur Falls, Woodland Cemetery) already use the correct modern `"This land enters tapped..."` wording. Only Hinterland Harbor still has the outdated phrasing.
2. **Doc comment uses outdated wording (line 7)**: Says `Hinterland Harbor enters the battlefield tapped` instead of `This land enters tapped`.
3. **Thin test coverage**: Only one test (`hinterland_harbor_card_data`) verifying that the card type is Land. No behavioral tests for ETB tapped/untapped logic or mana production. The sibling checkland Clifftop Retreat has four tests including ETB behavior and mana abilities.

### Tricky interactions checked (min 3)
1. **Dual lands with Forest/Island subtypes**: The check uses subtypes (`o.subtypes.iter().any(|s| s == "Forest")`), not card names. A Breeding Pool (Forest Island) or Tropical Island would correctly satisfy the condition. Verified at lines 21-23.
2. **Opponent's lands do not count**: `objects_in_zone(Zone::Battlefield, controller)` scopes the check to only the controller's permanents. An opponent's Forest does not allow untapped entry. Correct.
3. **Self-exclusion on ETB**: The `o.id != object_id` guard at line 20 prevents Hinterland Harbor from counting itself during the ETB check. While Hinterland Harbor has no land subtypes anyway (it is not a Forest or Island), this is a correct defensive check consistent with the other checklands.
4. **Mana ability availability**: Mana abilities are only offered when the card is on the battlefield and untapped (line 57). Correct gating.

### Test coverage
- `hinterland_harbor_card_data`: Verifies card type is Land. No other tests.
- **Missing**: No test for ETB tapped without matching land, no test for ETB untapped with Forest/Island, no test for mana production, no test verifying opponent's lands don't count.
