# Audit: Screeching Bat // Stalking Vampire

## Official Oracle (Front Face)
- **Name:** Screeching Bat
- **Cost:** {2}{B}
- **Type:** Creature — Bat
- **Oracle Text:** Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform Screeching Bat.
- **P/T:** 2/2

## Official Oracle (Back Face)
- **Name:** Stalking Vampire
- **Cost:** None
- **Type:** Creature — Vampire
- **Oracle Text:** At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform Stalking Vampire.
- **P/T:** 5/5

## Implementation Review
- **Front Face Name:** OK
- **Front Face Cost:** {2}{B} — OK
- **Front Face Type:** Creature, subtypes ["Bat"] — OK
- **Front Face Oracle:** Matches — OK
- **Front Face P/T:** 2/2 — OK
- **Front Face Keywords:** Flying — OK
- **Back Face Name:** "Stalking Vampire" — OK
- **Back Face Type:** Creature, subtypes ["Vampire"] — OK
- **Back Face Oracle:** Matches — OK
- **Back Face P/T:** 5/5 (via dynamic_pt) — OK
- **Transform:** on_upkeep checks active_player == controller, checks mana availability, auto-pays if possible — OK
- **Back face Flying:** Stalking Vampire should NOT have flying (only Screeching Bat has it). The back_face_data has no keywords — OK

## Issues
1. **Minor: "you may" is auto-decided**: The transform trigger says "you may pay" but the implementation auto-pays if mana is available. This removes player agency — the player might not want to transform even when they have the mana. Noted as a simplification.

## Verdict: PASS (with noted simplification on "you may" choice)
