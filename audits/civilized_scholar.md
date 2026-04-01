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

## Audit — 2026-04-01 09:00

**Scryfall Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap Civilized Scholar, then transform it.
**Scryfall Oracle text (back)**: At the beginning of your end step, if Homicidal Brute didn't attack this turn, tap Homicidal Brute, then transform it.
**Scryfall type line**: Creature — Human Advisor // Creature — Human Mutant
**Status**: ISSUE

Findings:
- Mana cost {2}{U}: correct.
- Front face: Creature, subtypes Human/Advisor, P/T 0/1: correct.
- Back face (via back_face_data): Creature, subtypes Human/Mutant, P/T 5/1: correct.
- Activated ability: {T} draw then discard, if creature discarded untap + transform: correct.
- dynamic_pt returns (5,1) when transformed: correct.
- End step: checks is_transformed, active_player == controller, and attacked_this_turn flag: correct.
- on_attacks sets "attacked_this_turn" card_state flag: correct.
- End step clears "attacked_this_turn" flag: correct.
- ISSUE 1 (FIXED from prior audit): Homicidal Brute now correctly taps before transforming back (line 159: `obj.tapped = true`). The prior audit flagged this as missing but it is now present.
- triggered_abilities vec declares TriggerKind::Attacks and TriggerKind::EndStep: correct, matches the on_attacks and on_end_step hooks.
- back_face_data triggered_abilities is empty -- the EndStep trigger is declared on the front face card_data instead. This is acceptable since the same struct handles both faces.
- Anti-pattern check: No spell resolution (creature enters battlefield via normal cast flow). No move_object to graveyard for spells.
- No CombatDamageDealt misuse.
- No missing token subtypes (no tokens created).
- Tests found in tier15_cards.rs.
