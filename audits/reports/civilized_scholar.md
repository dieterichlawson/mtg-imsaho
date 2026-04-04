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

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute?utm_source=api
**Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Oracle text (back)**: At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Front P/T**: 0/1
**Back P/T**: 5/1
**Rulings**:
- [2011-09-22] You don't have priority between untapping Civilized Scholar and transforming it.
- [2011-09-22] If Civilized Scholar attacks, and later in the turn it transforms, Homicidal Brute's last ability won't trigger.
- [2011-09-22] You'll tap and transform Homicidal Brute even if it couldn't attack.
**Status**: ISSUE

### Code issues

1. **Automatic discard selection** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, lines 110-115):
   - Oracle text says: `{T}: Draw a card, then discard a card.`
   - Code does: Automatically prefers discarding a creature card via `hand.iter().find(|(_, is_creature)| *is_creature).or(hand.first())`. The player should choose which card to discard. This forces a transform whenever a creature card is in hand, removing player agency over whether to transform.

### Tricky interactions checked
- Draw then discard order: PASS (draws first at line 101, then discards)
- Untap then transform (no priority between): PASS (lines 129-131 set tapped=false then is_transformed=true atomically, matching ruling)
- End step transform back only on controller's end step: PASS (line 150 checks `state.active_player != controller`)
- End step taps before transforming back: PASS (line 159 sets `obj.tapped = true` before `is_transformed = false`)
- Attack flag tracking: PASS (on_attacks sets flag at line 141, on_end_step checks and clears at lines 154-168)
- Ruling about Scholar attacking then transforming: PASS (the attack flag is set on the creature regardless of face, so if it attacked as Scholar and later transforms, the flag persists)
- Ruling about tapping/transforming even if couldn't attack: PASS (end step transform-back is unconditional if didn't attack, line 157 just checks the flag)
- Discard event emitted: PASS (line 118)

### Test coverage
- Draw, discard creature, transform: `tier15_cards.rs:1353` (civilized_scholar_draw_discard_creature_transforms)
- Discard non-creature (should not transform): NOT TESTED
- Homicidal Brute transforms back on end step if didn't attack: NOT TESTED
- Homicidal Brute does NOT transform back if attacked: NOT TESTED
- Ruling: no priority between untap and transform: NOT TESTED (but implemented correctly)
- Ruling: Scholar attacks then transforms, Brute ability doesn't trigger: NOT TESTED

## Audit — 2026-04-01 13:35

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute?utm_source=api
**Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Oracle text (back)**: At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Front P/T**: 0/1
**Back P/T**: 5/1
**Rulings**:
- [2011-09-22] You don't have priority between untapping Civilized Scholar and transforming it.
- [2011-09-22] If Civilized Scholar attacks, and later in the turn it transforms, Homicidal Brute's last ability won't trigger.
- [2011-09-22] You'll tap and transform Homicidal Brute even if it couldn't attack.
**Status**: PASS

### Code issues
No issues found.

The previous audit flagged "Automatic discard selection" as an issue. This has been fixed. The current code (lines 104-142) presents a `ChooseCardFromHand` choice to the player when multiple cards are in hand. When only one card is in hand, it auto-discards (correct, no choice needed). The `on_discard_choice` callback (line 145) checks whether the discarded card was a creature and handles untap+transform.

### Tricky interactions checked
- Draw then discard order: PASS (draws first at line 102, then discards)
- Player choice for discard: PASS (lines 131-142 present `ChooseCardFromHand` when multiple cards in hand)
- Untap then transform (no priority between): PASS (lines 123-126 set tapped=false then is_transformed=true atomically)
- End step transform back only on controller's end step: PASS (line 171 checks `state.active_player != controller`)
- End step taps before transforming back: PASS (line 180 sets `obj.tapped = true` before `is_transformed = false`)
- Attack flag tracking: PASS (on_attacks sets flag at line 162, on_end_step checks and clears at lines 175-189)
- Ruling: Scholar attacks then transforms, Brute doesn't trigger: PASS (attack flag persists regardless of face)
- Ruling: tapping/transforming even if couldn't attack: PASS (end step only checks the attack flag, not whether it could attack)
- Creature card detection via `power.is_some()`: PASS (consistent with codebase conventions)

### Test coverage
- Draw, discard creature, transform: `tier15_cards.rs:1356` (civilized_scholar_discard_creature_transforms)
- Discard non-creature, no transform: `tier15_cards.rs:1397` (civilized_scholar_discard_noncreature_no_transform)
- Homicidal Brute transforms back on end step if didn't attack: NOT TESTED
- Homicidal Brute does NOT transform back if attacked: NOT TESTED
- Ruling: no priority between untap and transform: NOT TESTED (implemented correctly)
- Ruling: Scholar attacks then transforms, Brute ability doesn't trigger: NOT TESTED

## Audit — 2026-04-01 18:30

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute?utm_source=api
**Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Oracle text (back)**: At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Front P/T**: 0/1
**Back P/T**: 5/1
**Rulings**:
- [2011-09-22] You don't have priority between untapping Civilized Scholar and transforming it.
- [2011-09-22] If Civilized Scholar attacks, and later in the turn it transforms, Homicidal Brute's last ability won't trigger.
- [2011-09-22] You'll tap and transform Homicidal Brute even if it couldn't attack.
**Status**: PASS

### Code issues
No issues found.

The discard choice is now properly presented to the player via `ChooseCardFromHand` (lines 131-142) when multiple cards are in hand. Auto-discard only happens with a single card in hand (lines 107-130), which is correct. The `on_discard_choice` callback (line 145) handles the creature check and transform logic.

Creature card detection uses `o.power.is_some()` (lines 113, 147) rather than `o.card_types.contains(&CardType::Creature)`. This is a minor semantic shortcut -- it works correctly for all cards in the Innistrad set but would fail for hypothetical non-creature cards with power (e.g., Vehicles). Acceptable for this card pool.

### Tricky interactions checked
- Draw then discard order: PASS (draws first at line 102, then discards)
- Player choice for discard: PASS (lines 131-142 present `ChooseCardFromHand` when multiple cards in hand)
- Untap then transform (no priority between): PASS (lines 123-126 set tapped=false then is_transformed=true atomically, matching ruling)
- End step transform back only on controller's end step: PASS (line 171 checks `state.active_player != controller`)
- End step taps before transforming back: PASS (line 180 sets `obj.tapped = true` before `is_transformed = false`)
- Attack flag tracking: PASS (on_attacks sets flag at line 162, on_end_step checks and clears at lines 175-189)
- Ruling: Scholar attacks then transforms, Brute doesn't trigger: PASS (attack flag persists regardless of face)
- Ruling: tapping/transforming even if couldn't attack: PASS (end step only checks the attack flag, not whether it could attack)
- Discard event emitted: PASS (lines 115-118)
- triggered_abilities declarations match hooks: PASS (TriggerKind::Attacks for on_attacks, TriggerKind::EndStep for on_end_step)

### Test coverage
- Draw, discard creature, transform: `tier15_cards.rs:1356` (civilized_scholar_discard_creature_transforms)
- Discard non-creature, no transform: `tier15_cards.rs:1397` (civilized_scholar_discard_noncreature_no_transform)
- Homicidal Brute transforms back on end step if didn't attack: NOT TESTED
- Homicidal Brute does NOT transform back if attacked: NOT TESTED
- Ruling: no priority between untap and transform: NOT TESTED (implemented correctly)
- Ruling: Scholar attacks then transforms, Brute ability doesn't trigger: NOT TESTED

## Audit — 2026-04-01 20:00

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute?utm_source=api
**Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Oracle text (back)**: At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Front P/T**: 0/1
**Back P/T**: 5/1
**Rulings**:
- [2011-09-22] You don't have priority between untapping Civilized Scholar and transforming it.
- [2011-09-22] If Civilized Scholar attacks, and later in the turn it transforms, Homicidal Brute's last ability won't trigger.
- [2011-09-22] You'll tap and transform Homicidal Brute even if it couldn't attack.
**Status**: PASS

### Code issues
No issues found.

All card data matches oracle text. The discard choice is properly presented to the player via `ChooseCardFromHand` (lines 131-142) when multiple cards are in hand. Auto-discard only occurs with a single card in hand (lines 107-130), which is correct. The `on_discard_choice` callback (line 145) checks whether the discarded card was a creature and handles untap+transform. End step correctly taps Homicidal Brute before transforming back (line 180).

Creature card detection uses `o.power.is_some()` (lines 113, 147) rather than `o.card_types.contains(&CardType::Creature)`. This is a minor semantic shortcut -- it works correctly for all cards in the Innistrad set.

### Tricky interactions checked
- Draw then discard order: PASS (draws first at line 102, then discards)
- Player choice for discard: PASS (lines 131-142 present `ChooseCardFromHand` when multiple cards in hand)
- Untap then transform (no priority between): PASS (lines 123-126 set tapped=false then is_transformed=true atomically, matching ruling)
- End step transform back only on controller's end step: PASS (line 171 checks `state.active_player != controller`)
- End step taps before transforming back: PASS (line 180 sets `obj.tapped = true` before `is_transformed = false`)
- Attack flag tracking: PASS (on_attacks sets flag at line 162, on_end_step checks and clears at lines 175-189)
- Ruling: Scholar attacks then transforms, Brute doesn't trigger: PASS (attack flag persists regardless of face)
- Ruling: tapping/transforming even if couldn't attack: PASS (end step only checks the attack flag, not whether it could attack)
- Discard event emitted: PASS (lines 115-118)
- triggered_abilities declarations match hooks: PASS (TriggerKind::Attacks for on_attacks, TriggerKind::EndStep for on_end_step)

### Test coverage
- Draw, discard creature, transform: `tier15_cards.rs:1474` (civilized_scholar_discard_creature_transforms)
- Discard non-creature, no transform: `tier15_cards.rs:1515` (civilized_scholar_discard_noncreature_no_transform)
- Homicidal Brute transforms back on end step if didn't attack: NOT TESTED
- Homicidal Brute does NOT transform back if attacked: NOT TESTED
- Ruling: no priority between untap and transform: NOT TESTED (implemented correctly)
- Ruling: Scholar attacks then transforms, Brute ability doesn't trigger: NOT TESTED
- LLM card knowledge: NOT PRESENT

## Audit — 2026-04-01 14:49

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute?utm_source=api
**Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Oracle text (back)**: At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Front P/T**: 0/1
**Back P/T**: 5/1
**Rulings**:
- [2011-09-22] You don't have priority between untapping Civilized Scholar and transforming it.
- [2011-09-22] If Civilized Scholar attacks, and later in the turn it transforms, Homicidal Brute's last ability won't trigger.
- [2011-09-22] You'll tap and transform Homicidal Brute even if it couldn't attack.
**Status**: PASS

### Code issues
No issues found.

Card data matches oracle text for both faces. Front face: {2}{U}, Creature - Human Advisor, 0/1. Back face via `back_face_data()`: Creature - Human Mutant, 5/1, with `dynamic_pt` returning (5,1) when transformed. Activated ability at ability_index 0: free mana cost, `requires_tap: true`, draws a card then presents discard choice via `ChooseCardFromHand` when multiple cards are in hand (lines 131-142). Auto-discard when only one card (lines 107-130). The `on_discard_choice` callback (line 145) checks if discarded card was a creature (`power.is_some()`) and handles untap+transform. End step (line 166) correctly checks `is_transformed`, `active_player == controller`, and the `attacked_this_turn` flag. Taps before transforming back (line 180: `obj.tapped = true`). Attack flag set in `on_attacks` (line 162) and cleared after check (line 188-189). `triggered_abilities` declares `TriggerKind::Attacks` and `TriggerKind::EndStep`, matching the `on_attacks` and `on_end_step` hooks.

### Tricky interactions checked
- Draw then discard order: PASS (draws first at line 102, then discards)
- Player choice for discard: PASS (lines 131-142 present `ChooseCardFromHand` when multiple cards in hand)
- Untap then transform (no priority between, per ruling): PASS (lines 123-126 set tapped=false then is_transformed=true atomically)
- End step transform back only on controller's end step: PASS (line 171 checks `state.active_player != controller`)
- End step taps before transforming back: PASS (line 180 sets `obj.tapped = true` before `is_transformed = false`)
- Attack flag tracking: PASS (on_attacks sets flag at line 162, on_end_step checks at line 175 and clears at lines 188-189)
- Ruling: Scholar attacks then transforms, Brute ability won't trigger: PASS (attack flag persists regardless of face)
- Ruling: tapping/transforming Brute even if couldn't attack: PASS (end step only checks the attack flag, not ability to attack)
- Discard event emitted: PASS (lines 115-118)
- triggered_abilities declarations match hooks: PASS (TriggerKind::Attacks for on_attacks, TriggerKind::EndStep for on_end_step)
- Creature card detection via `power.is_some()`: PASS (consistent with engine conventions; works for all Innistrad cards)

### Test coverage
- Draw, discard creature, transform: `tier15_cards.rs:1518` (civilized_scholar_discard_creature_transforms)
- Discard non-creature, no transform: `tier15_cards.rs:1559` (civilized_scholar_discard_noncreature_no_transform)
- Homicidal Brute transforms back on end step if didn't attack: NOT TESTED
- Homicidal Brute does NOT transform back if attacked: NOT TESTED
- Ruling: no priority between untap and transform: NOT TESTED (implemented correctly)
- Ruling: Scholar attacks then transforms, Brute ability doesn't trigger: NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute?utm_source=api
**Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Oracle text (back)**: At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Front P/T**: 0/1
**Back P/T**: 5/1
**Rulings**:
- [2011-09-22] You don't have priority between untapping Civilized Scholar and transforming it.
- [2011-09-22] If Civilized Scholar attacks, and later in the turn it transforms, Homicidal Brute's last ability won't trigger.
- [2011-09-22] You'll tap and transform Homicidal Brute even if it couldn't attack.
**Status**: ISSUE

### Code issues

1. **Back face oracle_text field is incorrect** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, line 60):
   - Oracle text says: `"At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it."`
   - Code has: `"At the beginning of your end step, if Homicidal Brute didn't attack this turn, transform Homicidal Brute."`
   - Two problems: (a) missing "tap this creature, then" before "transform", and (b) uses old printed card-name wording ("Homicidal Brute") instead of current oracle wording ("this creature"/"it").
   - **Behavior is correct** -- the `on_end_step` handler does tap before transforming (line 180: `obj.tapped = true`). This is a text-only issue in the `oracle_text` field.

2. **Front face oracle_text field uses old wording** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, line 33):
   - Oracle text says: `"...untap this creature, then transform it."`
   - Code has: `"...untap Civilized Scholar, then transform Civilized Scholar."`
   - Uses card name where current oracle uses "this creature" / "it". Minor text-only discrepancy; behavior is correct.

3. **No LLM knowledge entry**: No entry found in `mtg-player/src/llm.rs` for Civilized Scholar or Homicidal Brute.

### Tricky interactions checked
- Draw then discard order: PASS (draws first at line 102 via `draw_cards`, then discards)
- Player choice for discard: PASS (lines 131-142 present `ChooseCardFromHand` when multiple cards in hand; auto-discard only with single card at lines 107-130)
- Untap then transform (no priority between, per ruling): PASS (lines 123-126 set `tapped=false` then `is_transformed=true` atomically)
- End step transform back only on controller's end step: PASS (line 171 checks `state.active_player != controller`)
- End step taps before transforming back: PASS (line 180 sets `obj.tapped = true` before `is_transformed = false`)
- Attack flag tracking: PASS (`on_attacks` sets "attacked_this_turn" flag at line 162; `on_end_step` checks at line 175 and clears at lines 188-189)
- Ruling: Scholar attacks then transforms, Brute ability won't trigger: PASS (attack flag is set on the object regardless of which face is up, so the flag persists after transform)
- Ruling: tapping/transforming Brute even if couldn't attack: PASS (end step only checks the attack flag, not whether the creature could have attacked)
- Discard event emitted: PASS (lines 115-118)
- triggered_abilities declarations match hooks: PASS (front face declares `TriggerKind::Attacks` and `TriggerKind::EndStep`; `on_end_step` guards with `is_transformed` check so the EndStep trigger only has effect on back face)
- Creature card detection via `power.is_some()`: PASS (consistent with engine conventions)

### Test coverage
- Draw, discard creature, transform: `tier15_cards.rs` (civilized_scholar_discard_creature_transforms)
- Discard non-creature, no transform: `tier15_cards.rs` (civilized_scholar_discard_noncreature_no_transform)
- Homicidal Brute transforms back on end step if didn't attack: NOT TESTED
- Homicidal Brute does NOT transform back if attacked: NOT TESTED
- Ruling: no priority between untap and transform: NOT TESTED (implemented correctly)
- Ruling: Scholar attacks then transforms, Brute ability doesn't trigger: NOT TESTED
- LLM card knowledge: NOT PRESENT

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
// Homicidal Brute: At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked (min 3)
- Homicidal Brute end-step transform back: PASS
- Attack flag tracking across faces: PASS
- No priority between untap and transform: PASS
- Ruling: Scholar attacks then transforms, Brute ability won't trigger: PASS

### Test coverage
- Discard creature triggers transform: TESTED (civilized_scholar_discard_creature_transforms)
- Discard non-creature does not transform: TESTED (civilized_scholar_discard_noncreature_no_transform)
- Homicidal Brute transforms back on end step if didn't attack: NOT TESTED
- Homicidal Brute does NOT transform back if attacked: NOT TESTED

## Audit — 2026-04-02 20:41

**Oracle text source**: Scryfall API (oracle cache, cached 2026-04-01)
**Oracle text**:
- Front (Civilized Scholar): `{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.`
- Back (Homicidal Brute): `At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.`
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**Status**: PASS

### Code issues
No issues found. All card data matches oracle text exactly:
- Front face: "Civilized Scholar", {2}{U}, Creature — Human Advisor, 0/1. Matches.
- Back face: "Homicidal Brute", Creature — Human Mutant, 5/1 (via `dynamic_pt`). Matches.
- Activated ability: tap to draw then discard, creature check triggers untap+transform. Correctly only available on front face (`!o.is_transformed` guard at line 79).
- End-step trigger: checks `is_transformed` and `active_player == controller`, taps then transforms back if didn't attack. Matches oracle "tap this creature, then transform it."
- Attack flag set in `on_attacks` (line 162), checked and cleared in `on_end_step` (lines 175-189). Flag persists through transform (only `tapped`/`is_transformed`/`name` change).
- Untap and transform happen atomically (lines 124-126 and 149-152) with no priority pass, per ruling.
- Creature detection uses `power.is_some()` (lines 113, 147) rather than `card_types.contains(Creature)`. Functionally equivalent for this card pool but less robust in general. Minor style note, not a bug.

### Tricky interactions checked (min 3)
1. **No priority between untap and transform** (ruling): Implementation is atomic in a single code block (lines 123-127 in auto-discard path, lines 149-153 in callback path). No awaiting action or priority pass between them. PASS.
2. **Scholar attacks then transforms -- Brute ability won't trigger** (ruling): `on_attacks` sets "attacked_this_turn" flag regardless of which face is up (line 162). `on_end_step` checks this flag (line 176). If Scholar attacked and later transforms to Brute in the same turn, the flag is set and Brute won't transform back. PASS.
3. **Homicidal Brute taps and transforms even if it couldn't attack** (ruling): `on_end_step` only checks the "attacked_this_turn" flag (line 176), not whether the creature was able to attack. If it didn't attack for any reason (summoning sickness, tapped, etc.), it still taps and transforms back. PASS.
4. **Auto-discard with 1 card in hand** vs **choice with multiple cards**: When hand has 1 card after draw, auto-discards (lines 110-130). When hand has 2+ cards, presents `ChooseCardFromHand` (lines 131-142) and handles result in `on_discard_choice` (lines 145-157). Both paths emit the Discarded event and check for creature. PASS.

### Test coverage
- `civilized_scholar_discard_creature_transforms`: draws Grizzly Bears, discards it, verifies transform to Homicidal Brute and untapped state. PASS.
- `civilized_scholar_discard_noncreature_no_transform`: draws Doom Blade, discards it, verifies no transform and remains tapped. PASS.
- Missing tests: end-step transform back, attack flag preventing transform back, auto-discard path (1 card in hand).
