## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\nWhenever Falkenrath Noble or another creature dies, target player loses 1 life and you gain 1 life.
**Scryfall type line**: Creature — Vampire Noble
**Status**: ISSUE

- Mana cost {3}{B}: correct.
- Type Creature, subtypes Vampire Noble: correct.
- Power/Toughness 2/2: correct.
- Keywords: Flying: correct.
- Triggers: SelfDies + AnyCreatureDies: correct combination.
- Life drain logic (opponent loses 1, you gain 1): correct.
- LifeChanged events emitted for both sides: correct.

**Issue — "target player" always auto-targets opponent.** The Oracle says "target player loses 1 life" which means the controller should be able to choose which player loses 1 life (could target themselves in rare edge cases, or matters for hexproof/shroud on players). The implementation hardcodes `state.opponent(controller)` as the drain target. This is an acceptable simplification for 2-player but technically incorrect for the targeting semantics.

- Tests exist in `tier3_cards.rs` (`falkenrath_noble_drains_on_any_death`), `bug_fixes.rs` (multiple tests), and `apnap.rs`.

## Audit — 2026-04-01

**Scryfall Oracle text**: Flying. Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Scryfall type line**: Creature — Vampire Noble
**Status**: ISSUE

1. **"Target player" auto-targets opponent**: Oracle says "target player loses 1 life" which means the player should choose the target. The code auto-targets the opponent in 2-player games. This is noted as a simplification but means the ability doesn't correctly target. File: `mtg-engine/src/cards/falkenrath_noble.rs`, line 59-68.
