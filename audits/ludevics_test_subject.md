# Audit: Ludevic's Test Subject // Ludevic's Abomination

## Oracle (Official)
### Front: Ludevic's Test Subject
- **Cost:** {1}{U}
- **Type:** Creature — Lizard Egg
- **Oracle:** Defender. {1}{U}: Put a hatchling counter on Ludevic's Test Subject. Then if there are five or more hatchling counters on it, remove all of them and transform Ludevic's Test Subject.
- **P/T:** 0/3

### Back: Ludevic's Abomination
- **Type:** Creature — Lizard Horror
- **Oracle:** Trample
- **P/T:** 13/13

## Implementation
- Front name: "Ludevic's Test Subject" -- CORRECT
- Front cost: {1}{U} -- CORRECT
- Front P/T: 0/3 -- CORRECT
- Front keywords: [Defender] -- CORRECT
- Front oracle text matches -- CORRECT
- Back name: "Ludevic's Abomination" -- CORRECT
- Back subtypes: ["Lizard", "Horror"] -- CORRECT
- Back P/T: 13/13 (via dynamic_pt) -- CORRECT
- Back keywords: [Trample] -- CORRECT
- Activated ability: {1}{U}, adds hatchling counter, transforms at 5 -- CORRECT
- Uses card_state for hatchling counter tracking -- OK (workaround)

## Issues
1. **ISSUE (minor):** Front face subtypes are ["Lizard"] but should be ["Lizard", "Egg"]. The official type line is "Creature — Lizard Egg".

## Verdict: PASS (with minor subtype issue)

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: (Front) Defender. {1}{U}: Put a hatchling counter on Ludevic's Test Subject. Then if there are five or more hatchling counters on it, remove all of them and transform Ludevic's Test Subject. (Back) Trample.
**Scryfall type line**: (Front) Creature -- Lizard Egg. (Back) Creature -- Lizard Horror.
**Status**: PASS

Findings:
1. **Mana cost {1}{U}**: Correct.
2. **Front face P/T 0/3**: Correct.
3. **Front face subtypes**: Code has `["Lizard", "Egg"]` (line 23). Previous audit said "Egg" was missing, but the current code includes it. This is now correct.
4. **Front keywords [Defender]**: Correct.
5. **Activated ability {1}{U}**: Correct cost. Adds hatchling counter, transforms at 5+. Logic correct.
6. **Back face name (Ludevic's Abomination)**: Correct.
7. **Back face subtypes ["Lizard", "Horror"]**: Correct per Scryfall.
8. **Back face P/T 13/13**: Correct (via `dynamic_pt`).
9. **Back face keywords [Trample]**: Correct.
10. **Hatchling counter tracking**: Uses `card_state` HashMap with "hatchling_counters" key. Functional workaround for lack of native counter type.
11. **No anti-patterns detected**: No spells, no damage, no triggered abilities needed.
12. **Tests**: Found in `mtg-engine/tests/tier15_cards.rs`.

No new issues found. Previous subtype issue appears resolved.

## Audit — 2026-04-01 14:13

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/64/ludevics-test-subject-ludevics-abomination) and MTG Salvation (https://www.mtgsalvation.com/cards/innistrad/19131-ludevics-abomination)
**Oracle text (front)**: Defender. {1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.
**Oracle text (back)**: Trample
**Type line (front)**: Creature — Lizard Egg
**Type line (back)**: Creature — Lizard Horror
**Mana cost**: {1}{U}
**P/T (front)**: 0/3
**P/T (back)**: 13/13
**Status**: PASS

Findings:
1. **Name**: "Ludevic's Test Subject" / "Ludevic's Abomination" -- correct.
2. **Mana cost {1}{U}**: Correct (Generic(1), Blue).
3. **Front face subtypes ["Lizard", "Egg"]**: Correct per Scryfall.
4. **Front face P/T 0/3**: Correct.
5. **Front face keywords [Defender]**: Correct.
6. **Activated ability {1}{U}**: Correct cost (Generic(1), Blue). Not restricted to once-per-turn (correct, can activate multiple times). Not sorcery-speed-only (correct, activated abilities can be used at instant speed). Only available when not transformed (line 66, correct).
7. **Hatchling counter logic**: Counter tracked via card_state. Increments, checks >= 5, removes all and transforms. Correct per oracle. Per rulings, the check happens as part of the ability's resolution, which this correctly implements.
8. **Back face subtypes ["Lizard", "Horror"]**: Correct per Scryfall.
9. **Back face P/T 13/13**: Correct (via `dynamic_pt`).
10. **Back face keywords [Trample]**: Correct.
11. **should_transform returns false**: Correct -- transformation is handled by the activated ability, not by a trigger.
12. **Tests**: Found in `mtg-engine/tests/tier15_cards.rs`. Tests verify 4 activations do not transform, 5th does, and back face stats are 13/13. Assertions correct.

No issues found.
