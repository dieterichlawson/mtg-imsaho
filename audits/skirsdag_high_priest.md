## Audit — 2026-04-01

**Scryfall Oracle text**: Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
**Scryfall type line**: Creature — Human Cleric
**Mana cost**: {1}{B}
**P/T**: 1/2
**Status**: ISSUE

**Issue: Oracle text mismatch.** The implementation oracle_text says "Activate only as a sorcery" but the actual Oracle text (post-errata) says "Activate only if a creature died this turn." The morbid condition is checked correctly in code (`creature_died_this_turn`), and the ability is also marked `sorcery_speed_only: true`. The original Innistrad printing did say "Activate this ability only any time you could cast a sorcery" in addition to the morbid condition, so both restrictions should apply. The implementation correctly enforces both, but the oracle_text string omits the morbid activation restriction text.

The functional behavior (morbid check + sorcery speed + tap self + tap two other creatures + create 5/5 flying Demon token) is correct.

- Tests: 4 tests in tier10_cards.rs covering card data, morbid creation, no-morbid prevention, and creature count requirement
