# Audit: Silver-Inlaid Dagger

## Oracle (Scryfall)
- **Name:** Silver-Inlaid Dagger
- **Cost:** {1}
- **Type:** Artifact -- Equipment
- **Oracle:** Equipped creature gets +2/+0. As long as equipped creature is a Human, it gets an additional +1/+0. Equip {2}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/silver_inlaid_dagger.rs`
- **Name:** Silver-Inlaid Dagger ✅
- **Cost:** {1} ✅
- **Type:** Artifact ✅
- **Subtypes:** Equipment ✅
- **Base effect:** ModifyPT +2/+0 on Attached ✅
- **Human bonus:** update_effects checks for Human subtype and grants +3/+0 instead ✅
- **Equip cost:** {2}, sorcery speed, targets own creature ✅
- **is_equipment:** set to true on resolve ✅

### Issue
- **MINOR:** The oracle text in the implementation says "it gets +3/+0 instead" but the actual oracle text is "it gets an additional +1/+0" (meaning +2/+0 base plus +1/+0 additional = +3/+0 total). The functional result is the same (+3/+0 to Humans), so no gameplay bug.
- **POTENTIAL:** The Human check uses `registry.card_data(o.card_id)` to look at the base subtypes. If a creature gains or loses the Human subtype through effects, this would not be detected. Minor edge case.

## Verdict: PASS -- functionally correct, minor oracle text wording difference

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Silver-Inlaid Dagger
- **Mana Cost:** {1}
- **Type:** Artifact — Equipment
- **Oracle Text:** Equipped creature gets +2/+0. / As long as equipped creature is a Human, it gets an additional +1/+0. / Equip {2}

### Card Data Audit
- **Name:** Correct ("Silver-Inlaid Dagger")
- **Cost:** Correct ({1})
- **Types:** Correct (Artifact, subtype Equipment)
- **Oracle Text String:** MISMATCH
  - **Oracle:** "As long as equipped creature is a Human, it gets an additional +1/+0."
  - **Code:** "As long as equipped creature is a Human, it gets +3/+0 instead."
  - The oracle grants +2/+0 base and an additional +1/+0 for Humans (total +3/+0). The code text says "+3/+0 instead". End result is same but wording differs.
- **Equip Cost:** Correct ({2})

### Behavior Audit
- **Base +2/+0:** Default continuous effect `ModifyPT { power: 2, toughness: 0 }`. Correct.
- **Human bonus:** `update_effects` sets instance effects to +3/+0 for Humans, +2/+0 for non-Humans. Net effect for Humans is +3/+0 total, matching oracle (+2 base + 1 additional).
- **Potential double-counting concern:** card_data has `continuous_effects` with +2/+0 AND `update_effects` sets `instance_continuous_effects`. If both are applied simultaneously, a Human would get +2 (card_data) + +3 (instance) = +5/+0, which would be incorrect. Depends on whether instance effects replace or supplement card_data effects.
- **Equip {2}:** Sorcery speed, targets creature you control. Correct.
- **Equipment setup:** `on_resolve` sets `is_equipment = true`. Correct.

### Result
**ISSUE** -- Oracle text string mismatch: code says `"it gets +3/+0 instead"` but oracle says `"it gets an additional +1/+0"`. Also potential double-counting if both `continuous_effects` and `instance_continuous_effects` are applied simultaneously (would yield +5/+0 for Humans instead of correct +3/+0).
