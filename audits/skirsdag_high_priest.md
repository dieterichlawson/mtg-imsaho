## Audit — 2026-04-01

**Scryfall Oracle text**: Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
**Scryfall type line**: Creature — Human Cleric
**Mana cost**: {1}{B}
**P/T**: 1/2
**Status**: ISSUE

**Issue: Oracle text mismatch.** The implementation oracle_text says "Activate only as a sorcery" but the actual Oracle text (post-errata) says "Activate only if a creature died this turn." The morbid condition is checked correctly in code (`creature_died_this_turn`), and the ability is also marked `sorcery_speed_only: true`. The original Innistrad printing did say "Activate this ability only any time you could cast a sorcery" in addition to the morbid condition, so both restrictions should apply. The implementation correctly enforces both, but the oracle_text string omits the morbid activation restriction text.

The functional behavior (morbid check + sorcery speed + tap self + tap two other creatures + create 5/5 flying Demon token) is correct.

- Tests: 4 tests in tier10_cards.rs covering card data, morbid creation, no-morbid prevention, and creature count requirement

## Audit — 2026-04-01

**Scryfall Oracle text**: Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
**Scryfall type line**: Creature — Human Cleric
**Status**: ISSUE

1. **Oracle text field is wrong** (skirsdag_high_priest.rs:25): Code says "Activate only as a sorcery." but Scryfall Oracle text says "Activate only if a creature died this turn." The morbid condition IS correctly enforced by the `creature_died_this_turn` check in `activated_abilities()`, but the oracle_text string is incorrect.
2. **Incorrect `sorcery_speed_only: true`** (skirsdag_high_priest.rs:60): The current Oracle text does NOT restrict activation to sorcery speed. The only restriction is the morbid condition. Setting `sorcery_speed_only: true` incorrectly prevents activation during opponent's turn or in response to spells.
