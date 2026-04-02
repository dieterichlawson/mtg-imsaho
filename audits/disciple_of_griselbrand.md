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
