## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever another Human creature dies, put a +1/+1 counter on Village Cannibals.
**Scryfall type line**: Creature — Human
**Scryfall mana cost**: {2}{B}
**Scryfall P/T**: 2/2
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {2}{B} — correct.
- Types: Creature — Human — correct.
- P/T: 2/2 — correct.
- Oracle text: Matches.
- Trigger: `on_any_creature_dies` checks self is on battlefield, then checks if the dead creature is a Human (via registry subtypes). Adds +1/+1 counter. Correct.
- **Note**: The Oracle says "another Human creature dies" — the "another" means any Human that isn't Village Cannibals itself. The implementation does not explicitly check `dead_id != self_id`, but since it checks `self.zone == Battlefield` and the dead creature has left the battlefield, Village Cannibals dying would fail the zone check. Correct in practice.
- **Note**: The trigger is not restricted to "you control" — any Human dying anywhere triggers it. The implementation correctly does NOT check controller, matching Oracle text.
- Tests: `village_cannibals_gains_counter_on_human_death` and `village_cannibals_ignores_non_human_death` in tier3_cards.rs.

No issues found.
