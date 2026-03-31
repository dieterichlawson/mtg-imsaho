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
