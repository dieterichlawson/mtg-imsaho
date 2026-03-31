# Audit: Civilized Scholar // Homicidal Brute

## Scryfall Reference
- **Front Face: Civilized Scholar**
  - **Cost:** {2}{U}
  - **Type:** Creature -- Human Advisor
  - **Oracle:** {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
  - **P/T:** 0/1

- **Back Face: Homicidal Brute**
  - **Cost:** (none)
  - **Type:** Creature -- Human Mutant
  - **Oracle:** At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
  - **P/T:** 5/1

## Implementation: `civilized_scholar.rs`
- **Front face name:** Civilized Scholar -- CORRECT
- **Cost:** {2}{U} -- CORRECT
- **Front subtypes:** ["Human", "Advisor"] -- CORRECT
- **Front P/T:** 0/1 -- CORRECT
- **Back face name:** Homicidal Brute -- CORRECT
- **Back subtypes:** ["Human", "Mutant"] -- CORRECT
- **Back P/T:** 5/1 -- CORRECT
- **Activated ability:** {T}: Draw, discard, if creature discarded -> untap + transform -- CORRECT
- **End step:** Transforms back if didn't attack -- CORRECT

## Issues
1. **ISSUE: Homicidal Brute should TAP then transform back, not just transform.** Oracle says "tap this creature, then transform it." The implementation only sets `is_transformed = false` and name without tapping it first.
2. **ISSUE: Triggered ability kind uses EndStep, but Scryfall says "end step" which is correct. However, the front face triggered_abilities includes TriggerKind::Attacks and TriggerKind::EndStep. The EndStep trigger on the back face is correct.**
