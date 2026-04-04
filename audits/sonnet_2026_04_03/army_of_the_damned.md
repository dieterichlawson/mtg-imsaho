## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Create thirteen tapped 2/2 black Zombie creature tokens.
Flashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Token creation count (thirteen): PASS - Code correctly creates exactly 13 tokens in a loop
- Token properties (2/2 black Zombie creatures that enter tapped): PASS - Tokens have correct power/toughness, color, subtype, and enter tapped via obj.tapped = true
- Flashback cost ({7}{B}{B}{B}): PASS - flashback_cost field correctly specifies 7 generic + 3 black mana
- Flashback exile behavior: PASS - Uses move_spell_after_resolve which checks cast_with_flashback flag to exile instead of graveyard
- Sorcery timing restrictions: PASS - Flashback follows normal sorcery timing per engine implementation
- Keywords field handling: PASS - Empty keywords vector is correct since Flashback is not in the Keyword enum, handled via flashback_cost field instead

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic token creation (13 tapped 2/2 black Zombie tokens): `mtg-engine/tests/tier12_cards.rs:59-82` 
- Flashback mechanics: NOT TESTED (for this specific card, though flashback system is tested with other cards in flashback.rs)
- Flashback cost verification: NOT TESTED
- Flashback exile after resolution: NOT TESTED
- Sorcery timing with flashback: NOT TESTED
- Token properties verification (color, subtype): TESTED in `mtg-engine/tests/tier12_cards.rs:77-81`