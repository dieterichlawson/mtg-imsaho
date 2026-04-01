## Audit — 2026-04-01

**Scryfall Oracle text**: Vampire creatures you control get +2/+0 and gain first strike until end of turn.
**Scryfall type line**: Instant
**Scryfall mana cost**: {1}{R}
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {1}{R} — correct.
- Type: Instant — correct.
- Oracle text: Matches.
- Resolution: Finds all Vampire creatures controlled by the caster, applies +2/+0 and first strike until end of turn. Correctly uses `until_end_of_turn_effects` for the power bonus and `until_end_of_turn_keywords` for first strike.
- **Minor note**: The implementation checks `obj.power.is_some()` as a creature heuristic, which is standard for this engine.
- Tests: `vampiric_fury_buffs_vampires` in innistrad_cards.rs.

No issues found.

## Audit — 2026-04-01

**Scryfall Oracle text**: Vampire creatures you control get +2/+0 and gain first strike until end of turn.
**Scryfall type line**: Instant
**Mana cost**: {1}{R}
**Status**: ISSUE

1. **Vampire tokens not detected** (`mtg-engine/src/cards/vampiric_fury.rs`, lines 44-46): The Vampire subtype check only looks at `registry.card_data(obj.card_id)`, not `obj.subtypes`. Vampire creature tokens would be missed. Should also check `obj.subtypes.iter().any(|s| s == "Vampire")`.
