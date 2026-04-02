# Audit: Sharpened Pitchfork

## Official Oracle
- **Name:** Sharpened Pitchfork
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle Text:** Equipped creature has first strike.\nAs long as equipped creature is a Human, it gets +1/+1.\nEquip {1}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2} — OK
- **Type:** Artifact, subtypes ["Equipment"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Continuous Effects:** GrantKeyword FirstStrike on Attached — OK
- **Human bonus:** update_effects dynamically adds ModifyPT +1/+1 when attached creature is Human — OK
- **Equip:** {1}, sorcery speed — OK
- **on_resolve:** Moves to battlefield, sets is_equipment — OK

## Issues
None found.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Sharpened Pitchfork
- **Mana Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle Text:** Equipped creature has first strike. / As long as equipped creature is a Human, it gets +1/+1. / Equip {1}

### Card Data Audit
- **Name:** Correct ("Sharpened Pitchfork")
- **Cost:** Correct ({2})
- **Types:** Correct (Artifact, subtype Equipment)
- **Oracle Text String:** Correct
- **Keywords:** None intrinsic; first strike granted to equipped creature. Correct.

### Behavior Audit
- **First strike:** Granted via `ContinuousEffect::GrantKeyword` in card_data. Correct.
- **Human bonus +1/+1:** `update_effects` checks if equipped creature is a Human, adds `ModifyPT { power: 1, toughness: 1 }` via instance effects. Correct.
- **Non-Human:** Only first strike, no P/T bonus. Correct.
- **Equip {1}:** Activated ability with Generic(1), sorcery speed, targets creature you control. Correct.
- **Equipment setup:** `on_resolve` sets `is_equipment = true`. Correct.

### Result
**PASS**
