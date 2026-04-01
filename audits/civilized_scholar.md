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

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute)
**Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap Civilized Scholar, then transform it.
**Oracle text (back)**: At the beginning of your end step, if Homicidal Brute didn't attack this turn, tap Homicidal Brute, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Status**: ISSUE

Findings:
- Mana cost {2}{U}: correct.
- Front face: Creature, subtypes Human/Advisor, P/T 0/1: correct.
- Back face (via back_face_data): Creature, subtypes Human/Mutant, P/T 5/1: correct.
- Activated ability: {T}, no mana cost, draw then discard, if creature discarded untap + transform: correct.
- dynamic_pt returns (5,1) when transformed: correct.
- End step: checks is_transformed, active_player == controller, and attacked_this_turn flag: correct.
- Taps before transforming back (line 159: obj.tapped = true): correct per oracle.
- on_attacks sets "attacked_this_turn" card_state flag: correct.
- End step clears "attacked_this_turn" flag after check: correct.
- triggered_abilities vec declares TriggerKind::Attacks and TriggerKind::EndStep: correct.
- ISSUE 1: The discard is automatic -- the code always prefers discarding a creature card (lines 111-114) rather than giving the player a choice of which card to discard. Oracle says "draw a card, then discard a card" which implies the controller chooses which card to discard. This matters because the player might want to discard a non-creature card to avoid transforming.
- ISSUE 2: The code checks if a card is a creature by `o.power.is_some()` (line 105). This heuristic could be wrong for cards that have power/toughness defined but are not creatures in their current zone, or for creature cards that might not have power set on the object. However, this is a reasonable simplification.
- Anti-pattern check: Uses move_object(discard_id, Zone::Graveyard) for the discard (line 117), which is correct for discarding a card (not a spell resolution). No spell-to-graveyard anti-pattern.
- No CombatDamageDealt misuse.
- No missing token subtypes (no tokens created).
- Tests: 1 test in tier15_cards.rs (civilized_scholar_draw_discard_creature_transforms). Minimal coverage -- only tests the transform case. No test for: discarding non-creature (should not transform), Homicidal Brute transforming back on end step, Homicidal Brute NOT transforming back if it attacked.

## Audit — 2026-04-01 14:38

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute)
**Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Oracle text (back)**: At the beginning of your end step, if Homicidal Brute didn't attack this turn, tap Homicidal Brute, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Front P/T**: 0/1
**Back P/T**: 5/1
**Status**: ISSUE

Findings:
1. **Name**: "Civilized Scholar" / "Homicidal Brute" -- correct.
2. **Mana cost {2}{U}**: Correct (`Generic(2), Blue`).
3. **Front face type**: Creature, subtypes Human/Advisor, P/T 0/1 -- correct.
4. **Back face type**: Creature, subtypes Human/Mutant, P/T 5/1 -- correct (dynamic_pt returns (5,1) when transformed).
5. **Activated ability**: {T}, draw then discard, if creature discarded untap + transform -- correct structure.
6. **End step transform back**: Checks is_transformed, active_player == controller, attacked_this_turn flag. Taps before transforming (line 159: `obj.tapped = true`). Correct per oracle: `tap Homicidal Brute, then transform it.`
7. **on_attacks**: Sets "attacked_this_turn" card_state flag (line 141). Correct.
8. **End step clears flag**: Line 167-169 removes "attacked_this_turn" after check. Correct.
9. **triggered_abilities**: Declares TriggerKind::Attacks and TriggerKind::EndStep. Matches on_attacks and on_end_step hooks.
10. **Discard event**: Emitted at line 118. Correct.
11. **No spell cleanup needed**: Creature enters battlefield via normal cast flow.
12. **Tests**: No dedicated test file found. Previously noted in tier15_cards.rs.

Issue:
- **Automatic discard selection** (file: `mtg-engine/src/cards/civilized_scholar.rs`, lines 111-114):
  - Oracle text says: `{T}: Draw a card, then discard a card.`
  - Code does: Automatically prefers discarding a creature card (`hand.iter().find(|(_, is_creature)| *is_creature).or(hand.first())`).
  - The player should choose which card to discard. This matters because the player may want to discard a non-creature card to avoid transforming. The code always forces the transform when a creature card is in hand.
