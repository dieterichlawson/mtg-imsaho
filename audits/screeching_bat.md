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

## Audit — 2026-04-01 12:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
**Oracle text (back)**: At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
**Type line (front)**: Creature — Bat
**Type line (back)**: Creature — Vampire
**Status**: ISSUE

### Code issues

1. **"You may" is auto-decided** (`screeching_bat.rs:77-95`)
   - Oracle text says: `"you may pay {2}{B}{B}. If you do, transform this creature."`
   - Code does: `// Auto-pay if the controller has enough mana (simplified "you may").` — automatically pays and transforms whenever mana is available, with no player choice presented. The comment explicitly acknowledges this as a simplification. Per CLAUDE.md memory "NEVER take silent shortcuts" and "Correctness over convenience", this should present a real choice.

2. **Oracle text uses old card name instead of "this creature"** (`screeching_bat.rs:23,46`)
   - Oracle text says (front): `"transform this creature"` / (back): `"transform this creature"`
   - Code oracle_text (front): `"transform Screeching Bat"` / (back): `"transform Stalking Vampire"`
   - This is a cosmetic mismatch from the 2023 templating update ("this creature" replaced specific card names). Not a functional issue.

### Tricky interactions checked
- Transform does not grant/remove Flying: PASS (back face has no keywords, front face has Flying keyword)
- Upkeep trigger only fires for controller's upkeep: PASS (line 73-75 checks `state.active_player != controller`)
- Transform toggles correctly in both directions: PASS (line 86 uses `!is_transformed`)
- No mana = no transform: PASS (line 84 checks `crate::mana::can_pay(pool, &cost)`)
- dynamic_pt returns correct values: PASS (5/5 when transformed, None when not)

### Test coverage
- Transform with mana: `tier15_cards.rs:774` (screeching_bat_transforms_at_upkeep_with_mana)
- Transform without mana: NOT TESTED
- Declining to transform when mana is available: NOT TESTED (and not possible due to issue #1)
- Transform back from Stalking Vampire to Screeching Bat: NOT TESTED
