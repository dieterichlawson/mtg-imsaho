# Audit: Disciple of Griselbrand

## Reference (Scryfall)
- **Name:** Disciple of Griselbrand
- **Cost:** {1}{B}
- **Type:** Creature -- Human Cleric
- **Oracle:** {1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
- **P/T:** 1/1

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{B})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Cleric)
- Oracle text: CORRECT
- P/T: CORRECT (1/1)
- Activated ability cost {1}: CORRECT
- Sacrifice a creature cost: CORRECT (SacrificeCost::SacrificeCreature)
- requires_tap: CORRECT (false)
- Life gain equals sacrificed creature's toughness: CORRECT (reads from CreatureDied event)

## Issues
None found.

---

## Audit (2026-04-02)

### Oracle Text (Scryfall)

> {1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.

Ruling (2011-09-22): The amount of life you gain is equal to the toughness of the creature as it last existed on the battlefield, not its toughness in the graveyard.

### Implementation Oracle Text (line 25)

> {1}, Sacrifice a creature: You gain life equal to that creature's toughness.

### Card Data

- **Name:** Disciple of Griselbrand -- CORRECT
- **Mana Cost:** {1}{B} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Human, Cleric -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Activated ability cost:** {1}, Sacrifice a creature -- CORRECT
- **requires_tap:** false -- CORRECT
- **sacrifice_cost:** SacrificeCost::SacrificeCreature -- CORRECT

### Detailed Analysis

1. **Oracle text mismatch (cosmetic):** The embedded oracle_text says `"You gain life equal to that creature's toughness"` but Scryfall says `"You gain life equal to the sacrificed creature's toughness."` This is a minor wording difference with no functional impact (missing "sacrificed" qualifier, missing trailing period).

2. **Toughness source -- uses `last_known_toughness` from `CreatureDied` event:** The `destroy()` function in `destruction.rs` (line 95) computes `last_known_toughness` via `state.effective_toughness(id, registry)`, which accounts for +1/+1 counters, -1/-1 counters, and until-end-of-turn effects. This correctly implements the ruling that toughness is "as it last existed on the battlefield." -- CORRECT

3. **Life gain via `LifeChanged` event:** The implementation (lines 57-61) sets `new_life = old + toughness` and pushes a `LifeChanged` event. -- CORRECT

4. **`.max(0)` guard on toughness (line 55):** If toughness is negative (e.g., from -1/-1 counters), life gain is clamped to 0. This is correct -- you would not lose life from sacrificing a creature with 0 or negative toughness.

5. **BUG: Most-recent `CreatureDied` event lookup is fragile (lines 49-53):** The code does `state.events.iter().rev().find_map(...)` looking for the most recent `CreatureDied` event. This works because the engine sacrifices the creature immediately before calling `on_activate_ability`, so the last `CreatureDied` is guaranteed to be from this activation's sacrifice. However, there is no validation that the event belongs to this specific activation (e.g., checking controller or matching a specific object). If the engine's ordering ever changes, this could silently pick up a stale event. **Low severity** -- currently correct by engine contract but fragile.

6. **BUG: No player choice for sacrifice target:** The engine (engine.rs line 1554-1562) auto-sacrifices the first creature found via `.find()` when `SacrificeCost::SacrificeCreature` is used. The oracle text says the controller should choose which creature to sacrifice. The code has a TODO comment acknowledging this. This means the engine may sacrifice Disciple of Griselbrand itself (a 1/1) instead of letting the player choose a higher-toughness creature. **Medium severity** -- this is an engine-level issue, not specific to this card.

### Tests

One test in `mtg-engine/tests/tier8_cards.rs` (`disciple_of_griselbrand_gains_life`, line 138): Creates a Disciple and a 2/5 creature, activates the ability, and asserts life gained > 0. The test acknowledges the auto-sacrifice ambiguity with a comment. The test is weak -- it doesn't assert the exact amount of life gained.

### Issues Summary

| # | Severity | Description |
|---|----------|-------------|
| 1 | Low | Oracle text wording: implementation says `"that creature's toughness"`, Scryfall says `"the sacrificed creature's toughness."` |
| 2 | Low | Event lookup for `CreatureDied` is fragile (no validation it belongs to this activation) |
| 3 | Medium | Engine auto-sacrifices first creature found instead of presenting player choice (engine-level, has TODO) |
| 4 | Low | Test is weak -- only checks `gained > 0`, not exact toughness value |

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:54

**Oracle text source**: Scryfall API (cached 2026-04-01) via `scripts/oracle_lookup.py`
**Oracle text**: `{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.`
**Type line**: `Creature — Human Cleric`
**Status**: PASS

### Code issues

1. **Oracle text wording (cosmetic, no functional impact):** The stored `oracle_text` field on line 25 reads `"You gain life equal to that creature's toughness."` while Scryfall says `"You gain life equal to the sacrificed creature's toughness."` The word "that" vs "the sacrificed" and a missing trailing period. No behavioral difference.

2. **No player choice for sacrifice target (engine-level, pre-existing TODO):** The engine (`engine.rs` ~line 1750) auto-picks the first creature for `SacrificeCost::SacrificeCreature` via `.find()`. The player should choose which creature to sacrifice. This is an engine limitation with an existing TODO, not specific to this card.

3. **Event lookup for `CreatureDied` is correct but fragile:** `on_activate_ability` uses `state.events.iter().rev().find_map()` to locate the most recent `CreatureDied` event. This works because `submit_action` clears events at the start (line 1450) and the sacrifice cost is paid before `on_activate_ability` is called (line 1758 before line 1802). No validation ties the event to this specific activation. Currently correct by engine contract.

### Tricky interactions checked (min 3)

1. **Toughness uses last-known battlefield value (ruling 2011-09-22):** The `destroy()` function in `destruction.rs` (line 95-98) computes `last_known_toughness` via `state.effective_toughness(id, registry)` before moving the creature to the graveyard. This accounts for +1/+1 counters, -1/-1 counters, aura/equipment bonuses, and until-end-of-turn effects. Correctly implements the ruling: "The amount of life you gain is equal to the toughness of the creature as it last existed on the battlefield, not its toughness in the graveyard."

2. **Sacrificing Disciple itself is permitted:** `SacrificeCost::SacrificeCreature` does not exclude the source permanent (unlike `SacrificeCost::SacrificeAnotherCreature`). The Disciple can be sacrificed to its own ability, which is correct -- the oracle text says "Sacrifice a creature", not "Sacrifice another creature."

3. **Negative toughness handling:** Line 55 uses `.max(0)` to clamp toughness, and line 57 checks `if toughness > 0` before granting life. A creature with 0 or negative toughness (e.g., from -1/-1 counters) correctly gains 0 life. Per MTG rules, you cannot gain a negative amount of life.

4. **Ability is not tap-dependent:** `requires_tap: false` is correct -- the ability costs {1} and a creature sacrifice, not a tap. The Disciple can use its ability even if it has summoning sickness or is already tapped.

5. **Can be activated at instant speed:** `sorcery_speed_only: false` and `once_per_turn: false` are both correct. The ability has no timing restrictions beyond normal activated ability rules.

### Test coverage

One test: `disciple_of_griselbrand_gains_life` in `mtg-engine/tests/tier8_cards.rs` (line 138). Creates a Disciple and a 2/5 creature, activates the ability with {1} mana, and asserts `gained > 0`. The test passes but is weak -- it does not assert the exact life gained (could be 1 or 5 depending on which creature the engine auto-sacrifices). No test for sacrificing the Disciple itself, no test for creatures with modified toughness.
