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
