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

---

# Audit: Rolling Temblor (2026-04-02)

## Oracle Text (Scryfall)
- **Name:** Rolling Temblor
- **Mana Cost:** {2}{R}
- **Type:** Sorcery
- **Oracle Text:** Rolling Temblor deals 2 damage to each creature without flying. Flashback {4}{R}{R}

## Card Data Verification
- **Name:** Correct ("Rolling Temblor")
- **Cost:** Correct ({2}{R})
- **Type:** Correct (Sorcery)
- **Flashback Cost:** Correct ({4}{R}{R})
- **Keywords:** Correct (none inherent)

## Behavior Verification
- **Target selection:** Correct — collects all creatures on the battlefield.
- **Flying filter:** Correct — uses `state.has_keyword(id, Keyword::Flying, registry)` to skip creatures with flying.
- **Damage:** Correct — marks 2 damage on each non-flying creature and emits `NonCombatDamageDealt` event.
- **Cleanup:** Correct — calls `move_spell_after_resolve`.

## Result: PASS
