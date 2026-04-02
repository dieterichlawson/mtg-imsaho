# Audit: Crossway Vampire

## Scryfall Reference
- **Name:** Crossway Vampire
- **Cost:** {1}{R}{R}
- **Type:** Creature -- Vampire
- **Oracle:** When this creature enters, target creature can't block this turn.
- **P/T:** 3/2
- **Keywords:** none

## Implementation: `crossway_vampire.rs`
- **Name:** Crossway Vampire -- CORRECT
- **Cost:** {1}{R}{R} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Vampire"] -- CORRECT
- **P/T:** 3/2 -- CORRECT
- **Keywords:** none -- CORRECT
- **Trigger:** EntersBattlefield -- CORRECT
- **Behavior:** Presents target choice for "can't block this turn" -- CORRECT

## Issues
None

---

## Re-Audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
> When this creature enters, target creature can't block this turn.

### Card Data Verification
| Field     | Oracle / Scryfall | Implementation | Status  |
|-----------|-------------------|----------------|---------|
| Name      | Crossway Vampire  | `"Crossway Vampire"` | CORRECT |
| Cost      | {1}{R}{R}         | `Generic(1), Red, Red` | CORRECT |
| Type      | Creature          | `CardType::Creature` | CORRECT |
| Subtypes  | Vampire           | `["Vampire"]` | CORRECT |
| P/T       | 3/2               | `power: Some(3), toughness: Some(2)` | CORRECT |
| Keywords  | (none)            | `vec![]` | CORRECT |

### ETB Trigger
- `triggered_abilities` contains one entry with `TriggerKind::EntersBattlefield` -- CORRECT.
- `on_enter_battlefield` calls `creature_targets(state)` which returns all creatures on the battlefield. Oracle says "target creature" with no "another" restriction, so including self is correct.
- Targeting is mandatory (`optional: false`), matching oracle (no "may"). CORRECT.
- Effect is `PendingEffect::CantBlockThisTurn`, which pushes the target ID to `state.until_end_of_turn_cant_block`. This list is:
  - Checked during blocker declaration in `combat.rs:611` to filter out creatures that can't block. CORRECT.
  - Cleared at end of turn in `engine.rs:2632`. CORRECT.

### Tests
- No dedicated test file found for Crossway Vampire.

### llm.rs
- Entry at line 138: `"Crossway Vampire ({1}{R}{R} creature 3/2): When it enters, target creature can't block this turn. Clears the way for attackers."` -- accurate summary.

### Issues Found
None. The implementation faithfully matches the oracle text.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, target creature can't block this turn.
**Type line**: Creature — Vampire
**Status**: ISSUE

### Code issues
Oracle text mismatch: code stores `"When Crossway Vampire enters the battlefield, target creature can't block this turn."` but current oracle text is `"When this creature enters, target creature can't block this turn."`. Behavior is correct; only the stored oracle_text string needs updating to match the modern Scryfall wording.

## Audit — 2026-04-02 (final-pass)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found. Oracle text field matches current Scryfall template.
