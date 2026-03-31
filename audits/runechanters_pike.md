# Audit: Runechanter's Pike

## Official Oracle
- **Name:** Runechanter's Pike
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle Text:** Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.\nEquip {2}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2} — OK
- **Type:** Artifact, subtypes ["Equipment"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Continuous Effects:** GrantKeyword FirstStrike on Attached — OK
- **dynamic_pt:** Counts instant/sorcery cards in controller's graveyard, returns (count, 0) — OK
- **Equip ability:** Equip {2}, sorcery speed, targets creature — OK
- **on_resolve:** Moves to battlefield, sets is_equipment — OK
- **on_activate_ability:** Attaches to target creature — OK

## Issues
1. **Minor: dynamic_pt counts from equipment's controller, not equipped creature's controller**: The dynamic_pt uses the equipment's own controller. If the equipment were somehow controlled by a different player than the equipped creature (unusual), this could differ. In practice this is fine.

## Verdict: PASS
