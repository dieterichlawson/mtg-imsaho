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
