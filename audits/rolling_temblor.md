# Audit: Rolling Temblor

## Official Oracle
- **Name:** Rolling Temblor
- **Cost:** {2}{R}
- **Type:** Sorcery
- **Oracle Text:** Rolling Temblor deals 2 damage to each creature without flying.\nFlashback {4}{R}{R}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2}{R} — OK
- **Type:** Sorcery — OK
- **Oracle Text:** "Rolling Temblor deals 2 damage to each creature without flying." — OK (flashback in separate field)
- **Flashback Cost:** {4}{R}{R} — OK
- **P/T:** N/A — OK
- **on_resolve:** Iterates all battlefield creatures, skips those with Flying keyword, marks 2 damage, emits NonCombatDamageDealt event — OK
- **NonCombatDamageDealt:** Correctly emitted for each damaged creature — OK
- **Flying check:** uses state.has_keyword which checks both static and granted keywords — OK

## Issues
None found.

## Verdict: PASS
