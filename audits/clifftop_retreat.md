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
