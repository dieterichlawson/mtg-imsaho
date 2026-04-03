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

## Audit — 2026-04-02 20:45
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: When this creature enters, target creature can't block this turn.
**Type line**: Creature — Vampire
**Status**: PASS

### Code issues
None. All card data fields (name, cost, types, subtypes, P/T, keywords, oracle text) match Scryfall exactly. The ETB trigger is correctly implemented as a mandatory target choice using `PendingEffect::CantBlockThisTurn`, which adds the target to `state.until_end_of_turn_cant_block`. This list is checked during blocker declaration (`combat.rs:611`) and cleared at end of turn (`engine.rs:3023`).

### Tricky interactions checked (min 3)
1. **Self-targeting**: Oracle says "target creature" (not "another creature"), so Crossway Vampire can target itself. `creature_targets(state)` correctly includes all creatures on the battlefield, including self.
2. **Mandatory targeting**: The ability has no "may" clause, so targeting is mandatory. Implementation passes `optional: false` to `present_target_choice`. When exactly one creature exists, the helper auto-applies the effect without prompting.
3. **End-of-turn cleanup**: The "can't block this turn" restriction expires at end of turn via `until_end_of_turn_cant_block.clear()` at `engine.rs:3023`. If Crossway Vampire enters during a second main phase, the target still can't block for the remainder of that turn (no combat phase remains, so effectively a no-op targeting choice, but mechanically correct).

### Test coverage
- No dedicated unit test for Crossway Vampire.
- The `CantBlockThisTurn` mechanism is covered by Nightbird's Clutches tests in `tests/flashback.rs:455-467` and `tests/card_mechanics.rs:182-189`, which verify the effect is applied and that affected creatures are excluded from `eligible_blockers`.
