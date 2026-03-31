# Audit: Geistcatcher's Rig

## Oracle Reference (Scryfall)
- Cost: {6}
- Type: Artifact Creature -- Construct
- P/T: 4/5
- Oracle: "When Geistcatcher's Rig enters the battlefield, you may have it deal 4 damage to target creature with flying."

## Implementation: geistcatchers_rig.rs

## Issues Found

No issues found. Name, cost ({6}), types (Artifact Creature), subtype (Construct), P/T (4/5), oracle text all match. The ETB ability is correctly implemented as optional ("you may") with target filtering for creatures with flying. Uses PendingEffect::DealDamage which properly handles damaged_by and NonCombatDamageDealt.

## Verdict: PASS
