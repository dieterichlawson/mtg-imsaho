## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target Spirit or enchantment.
**Scryfall type line**: Instant
**Scryfall mana cost**: {1}{W}
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {1}{W} — correct.
- Type: Instant — correct.
- Oracle text: Matches.
- Targeting: `is_valid_target` checks that the permanent is on the battlefield and is either an Enchantment (by card type) or a Spirit (by subtype). Correct.
- Target requirement uses `SubtypeOrCardType` filter — correct.
- Resolution: Uses `resolve_destroy` helper. Correct.
- Tests: `urgent_exorcism_destroys_spirit` in tier2_spells.rs.

No issues found.
