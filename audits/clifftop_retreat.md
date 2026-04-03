# Audit: Clifftop Retreat

## Scryfall Reference
- **Name:** Clifftop Retreat
- **Cost:** (none)
- **Type:** Land
- **Oracle:** This land enters tapped unless you control a Mountain or a Plains. {T}: Add {R} or {W}.
- **P/T:** N/A
- **Keywords:** none

## Implementation: `clifftop_retreat.rs`
- **Name:** Clifftop Retreat -- CORRECT
- **Cost:** None -- CORRECT
- **Type:** Land -- CORRECT
- **Subtypes:** none -- CORRECT
- **P/T:** N/A -- CORRECT
- **Keywords:** none -- CORRECT
- **ETB check:** Checks for Mountain or Plains subtypes on other lands -- CORRECT
- **Mana abilities:** Add {R} or {W} -- CORRECT

## Issues

### Issue 1: Oracle text uses outdated templating (cosmetic)

**Scryfall oracle text:**
> This land enters tapped unless you control a Mountain or a Plains.

**Implementation oracle_text (line 37 of clifftop_retreat.rs):**
> Clifftop Retreat enters the battlefield tapped unless you control a Mountain or a Plains.

The current official oracle text uses "This land enters tapped" (post-Bloomburrow templating update), but the implementation still uses the older "Clifftop Retreat enters the battlefield tapped" wording. The doc comment on line 7 has the same outdated phrasing. This is a text-only mismatch; the functional behavior (ETB tapped check, mana production) is correct.

### Verdict
No functional bugs. One cosmetic oracle text mismatch with modern templating.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: This land enters tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
**Type line**: Land
**Status**: ISSUE

### Code issues
Oracle text mismatch: code stores `"Clifftop Retreat enters the battlefield tapped unless you control a Mountain or a Plains."` but current oracle text is `"This land enters tapped unless you control a Mountain or a Plains."`. Behavior is correct; only the stored oracle_text string needs updating to match the modern Scryfall wording.

## Audit — 2026-04-02 (final-pass)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found. Oracle text field matches current Scryfall template.

## Audit — 2026-04-02 20:41

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/238/clifftop-retreat)
**Oracle text**: This land enters tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
**Type line**: Land
**Status**: PASS

### Code issues
None. All card data fields match oracle text. Checkland ETB logic is correct. Mana abilities produce the correct colors. Self-exclusion check (`o.id != object_id`) is present. Oracle text string matches current Scryfall wording.

### Tricky interactions checked (min 3)
1. **Dual lands with Mountain/Plains subtypes**: The check uses subtypes (not card names), so a Sacred Foundry (Mountain Plains) or Stomping Ground (Mountain Forest) would correctly satisfy the condition. Verified in code at line 21-22.
2. **Opponent's lands do not count**: `objects_in_zone(Zone::Battlefield, controller)` scopes the check to only the controller's permanents. An opponent's Mountain does not allow untapped entry. Correct.
3. **Self-exclusion on ETB**: The `o.id != object_id` guard at line 20 prevents Clifftop Retreat from counting itself during the ETB check. While Clifftop Retreat has no land subtypes anyway, this is a correct defensive check consistent with the other checklands.
4. **Multiple Clifftop Retreats**: A second Clifftop Retreat entering does not benefit from the first, since the first has no Mountain/Plains subtypes. Correct behavior.

### Test coverage
- `clifftop_retreat_card_data`: Verifies card data (cost=None, type=Land, oracle text contains key phrase)
- `clifftop_retreat_enters_tapped_without_matching_land`: ETB tapped when no Mountain/Plains present
- `clifftop_retreat_enters_untapped_with_mountain`: ETB untapped when Mountain is on battlefield
- `clifftop_retreat_produces_red_or_white`: Both mana abilities ({R} and {W}) are available
- Note: No test for Plains specifically enabling untapped entry (only Mountain tested), and no test verifying opponent's lands don't count. These are minor gaps; the logic is symmetric and correctly scoped.
