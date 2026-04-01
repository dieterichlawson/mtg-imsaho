## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever another creature you control dies, put a +1/+1 counter on Unruly Mob.
**Scryfall type line**: Creature — Human
**Scryfall mana cost**: {1}{W}
**Scryfall P/T**: 1/1
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {1}{W} — correct.
- Types: Creature — Human — correct.
- P/T: 1/1 — correct.
- Oracle text: Matches.
- Trigger: `on_any_creature_dies` correctly checks that self is on the battlefield and that the dead creature's controller matches self's controller. Note: does not explicitly exclude self dying (dead_id != self_id), but since it checks self is on the battlefield, if self dies it would no longer be on the battlefield, so this is fine.
- Tests: `unruly_mob_gains_counter_when_ally_dies` in tier3_cards.rs.

No issues found.

## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever another creature you control dies, put a +1/+1 counter on this creature.
**Scryfall type line**: Creature — Human
**P/T**: 1/1, **Mana cost**: {1}{W}
**Status**: PASS

No issues found.
