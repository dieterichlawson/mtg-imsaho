## Audit — 2026-04-02 21:23
**Oracle text source**: Scryfall API (cached 2026-04-01, via `scripts/oracle_lookup.py`)
**Oracle text**: Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
**Type line**: Creature — Demon
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked (min 3)
1. **Life loss vs damage**: PASS -- The engine implements "loses 1 life" as direct life subtraction (`old - 1` at engine.rs:2267-2268), not damage. No `NonCombatDamageDealt` event is emitted (compare with damage-dealing code at engine.rs:2200-2203 which does emit that event). This is correct: life loss cannot be prevented by damage prevention effects and does not trigger "whenever a source deals damage" abilities.
2. **Controller's upkeep only**: PASS -- `on_upkeep` checks `state.active_player != controller` at line 44, correctly ensuring the trigger only fires during the controller's own upkeep step, not during any other player's upkeep.
3. **Target any player (including self)**: PASS -- Code iterates all non-lost players (lines 48-51), allowing the controller to target themselves or any opponent. This matches "target player" semantics correctly.
4. **Non-optional targeting**: PASS -- `optional: false` at line 62 correctly reflects that this is a mandatory ability (no "may" in the oracle text). The controller must choose a target.
5. **Battlefield zone check**: PASS -- Both the trigger system dispatch and `on_upkeep` (lines 40-43) verify the demon is on the battlefield before executing. If removed before the trigger resolves, nothing happens.

### Test coverage
- Basic upkeep trigger (draw + lose 1 life, targeting self): `tier7_cards.rs:70` (`bloodgift_demon_draws_and_loses_life`)
- Targeting opponent: NOT TESTED
- Trigger not firing on opponent's upkeep: NOT TESTED
- Demon removed before trigger resolves: NOT TESTED

---

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
**Type line**: Creature — Demon
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:33

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
**Type line**: Creature — Demon
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Life loss vs damage: PASS — Engine implements "loses 1 life" as direct life subtraction (`old - 1`), not damage. This is correct; life loss cannot be prevented by damage prevention effects, and does not trigger "dealt damage" abilities.
- Controller's upkeep only: PASS — Code checks `state.active_player != controller` at line 44, correctly ensuring the trigger only fires on the controller's own upkeep, not every player's upkeep.
- Demon removed before resolution: PASS — Both the trigger system (`triggers.rs:955`) and `on_upkeep` (line 40-43) verify the demon is still on the battlefield before firing. If removed in response, the trigger does nothing.
- Target any player (including self): PASS — Code iterates all non-lost players (line 48-51), allowing the controller to target themselves or any opponent, matching "target player" semantics.
- Non-optional targeting: PASS — `optional: false` at line 62 correctly reflects that this is not a "may" ability; the controller must choose a target.

### Test coverage
- Basic upkeep trigger (draw + lose 1 life, targeting self): `tier7_cards.rs:70` (bloodgift_demon_draws_and_loses_life)
- Targeting opponent instead of self: NOT TESTED
- Trigger does not fire on opponent's upkeep: NOT TESTED
- Demon removed before trigger resolves: NOT TESTED
