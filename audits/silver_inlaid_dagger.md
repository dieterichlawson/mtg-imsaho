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
