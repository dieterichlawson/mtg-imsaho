# Audit: Cloistered Youth // Unholy Fiend

## Scryfall Reference
- **Front Face: Cloistered Youth**
  - **Cost:** {1}{W}
  - **Type:** Creature -- Human
  - **Oracle:** At the beginning of your upkeep, you may transform this creature.
  - **P/T:** 1/1

- **Back Face: Unholy Fiend**
  - **Cost:** (none)
  - **Type:** Creature -- Horror
  - **Oracle:** At the beginning of your end step, you lose 1 life.
  - **P/T:** 3/3

## Implementation: `cloistered_youth.rs`
- **Front face name:** Cloistered Youth -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Front subtypes:** ["Human"] -- CORRECT
- **Front P/T:** 1/1 -- CORRECT
- **Back face name:** Unholy Fiend -- CORRECT
- **Back subtypes:** ["Horror"] -- CORRECT
- **Back P/T:** 3/3 -- CORRECT
- **Upkeep:** Transforms front to back -- CORRECT
- **End step:** Loses 1 life when transformed -- CORRECT

## Issues
1. **ISSUE: Front face P/T is 1/1, but Scryfall says 1/1.** Wait -- actually the doc comment says "3/2" on line 6 but the card_data says power: Some(1), toughness: Some(1). Checking Scryfall: front face is 1/1. The doc comment on line 6 says "{1}{W} 3/2 Human" which is WRONG in the comment but the code uses 1/1 which is CORRECT. The dynamic_pt returns (3,3) when transformed which matches the back face. The comment is just misleading but the code is correct.

No functional issues.

## Audit — 2026-04-01 12:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: At the beginning of your upkeep, you may transform this creature.
**Oracle text (back)**: At the beginning of your end step, you lose 1 life.
**Type line (front)**: Creature — Human
**Type line (back)**: Creature — Horror
**Status**: ISSUE

### Code issues

1. **"You may" transform is auto-decided** (`cloistered_youth.rs:80-88`)
   - Oracle text says: `"At the beginning of your upkeep, you may transform this creature."`
   - Code does: automatically transforms at upkeep with no player choice — `if !is_transformed { obj.is_transformed = true; }`. The player cannot decline to transform.

2. **Spurious triggers due to misplaced triggered_abilities declarations** (`cloistered_youth.rs:28-43`)
   - Front face declares both `TriggerKind::Upkeep` ("may transform") and `TriggerKind::EndStep` ("lose 1 life")
   - Back face declares `triggered_abilities: vec![]` (empty)
   - Problem: When Cloistered Youth is NOT transformed, an EndStep trigger fires on the stack with description "lose 1 life" (doing nothing since `on_end_step` checks `is_transformed`). When transformed as Unholy Fiend, an Upkeep trigger fires on the stack with description "may transform" (doing nothing since `on_upkeep` checks `!is_transformed`).
   - The front face's `triggered_abilities` should only declare `TriggerKind::Upkeep`, and the back face's `triggered_abilities` should declare `TriggerKind::EndStep`.

3. **Misleading doc comment** (`cloistered_youth.rs:6`)
   - Comment says: `"Cloistered Youth {1}{W} 3/2 Human"`
   - Oracle P/T is: 1/1 (front), 3/3 (back)
   - Code card_data correctly uses `power: Some(1), toughness: Some(1)` — the comment is just wrong but the code is correct.

### Tricky interactions checked
- Upkeep trigger only fires for controller: PASS (line 78-79 checks `state.active_player != controller`)
- Life loss emits LifeChanged event: PASS (line 104)
- Back face P/T via dynamic_pt: PASS (returns (3,3) when transformed)
- Already-transformed creature does not re-transform at upkeep: PASS (line 80 checks `!is_transformed`)

### Test coverage
- Transform at upkeep: `tier15_cards.rs:740` (cloistered_youth_transforms_at_upkeep)
- Life loss at end step when transformed: `tier15_cards.rs:754` (unholy_fiend_drains_life_at_end_step)
- Declining to transform: NOT TESTED (and not possible due to issue #1)
- No life loss when NOT transformed at end step: NOT TESTED
- Spurious trigger visibility: NOT TESTED
