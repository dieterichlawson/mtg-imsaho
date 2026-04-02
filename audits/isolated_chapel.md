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
