## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Create thirteen tapped 2/2 black Zombie creature tokens.
Flashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Mana cost {5}{B}{B}{B}**: Code has `Generic(5)` + 3×`Colored(Black)` = {5}{B}{B}{B}. ✓
- **Flashback cost {7}{B}{B}{B}**: Code has `Generic(7)` + 3×`Colored(Black)` in `flashback_cost`. ✓
- **Thirteen tokens created**: `for _ in 0..13` loop creates exactly 13 tokens. ✓
- **Tokens enter tapped**: `obj.tapped = true` set after each `create_token_with_subtypes` call. Consistent with engine pattern used by Diregraf Ghoul and Geist of Saint Traft's angel. ✓
- **Token properties (2/2 black Zombie creature)**: `power=2, toughness=2, colors=[Black], card_types=[Creature], subtypes=["Zombie"]`. ✓
- **Flashback exile on resolve**: Card calls `state.move_spell_after_resolve(object_id)`, which reads `cast_with_flashback` flag and exiles if set. Engine sets `cast_with_flashback = true` before calling `on_resolve`. ✓
- **Flashback exile when countered**: Counterspell's `on_resolve` calls `state.move_spell_after_resolve(target_id)` (not `move_object(Graveyard)`), so `cast_with_flashback` is respected. Confirmed by `flashback_spell_countered_is_exiled` test in `flashback.rs`. ✓
- **Flashback exile when fizzled**: `stack.rs:83-84` calls `move_spell_after_resolve` on fizzle path too. ✓
- **Sorcery timing restriction for flashback**: Engine checks `is_sorcery_type` and requires `is_sorcery_speed` for graveyard flashback of sorceries. Matches ruling: "you can cast a sorcery using flashback only when you could normally cast a sorcery." ✓
- **move_spell_after_resolve called twice (double-move check)**: Card calls it in `on_resolve`; `stack.rs:107-110` calls it again only if `obj.zone == Zone::Stack` — after the card's own call the zone is already Exile or Graveyard, so no double-move. ✓
- **Parallel Lives interaction**: `create_token_with_subtypes` is called 13 times in a loop; each call individually applies the Parallel Lives doubling, so 1 Parallel Lives yields 26 total tokens. This is correct per MTG rules (each individual token creation event is doubled). ✓
- **"Flashback" absent from `keywords` vec**: Correct — Flashback is a keyword ability handled via `flashback_cost` field, not the `Keyword` enum. Not an issue per audit rules. ✓
- **Reminder text omitted from `oracle_text` field**: The implementation omits `(You may cast this card from your graveyard for its flashback cost. Then exile it.)`. Reminder text has no mechanical significance. ✓

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- 13 tapped 2/2 black Zombie tokens created on normal cast: `mtg-engine/tests/tier12_cards.rs:59` (`army_of_the_damned_creates_13_tapped_zombies`) ✓
- Flashback spell exiled after resolution (general mechanic): `mtg-engine/tests/flashback.rs:86` (`flashback_spell_is_exiled_after_resolve`) ✓
- Flashback offered from graveyard (general mechanic): `mtg-engine/tests/flashback.rs:23` (`flashback_offered_from_graveyard`) ✓
- Flashback not offered without sufficient mana: `mtg-engine/tests/flashback.rs:65` (`flashback_not_offered_without_mana`) ✓
- Flashback countered spell still exiled (general mechanic): `mtg-engine/tests/flashback.rs:129` (`flashback_spell_countered_is_exiled`) ✓
- Army of the Damned specifically cast via flashback and exiled: NOT TESTED (no card-specific flashback test)
- Parallel Lives doubling of Army of the Damned tokens: NOT TESTED (general Parallel Lives tested in `tier14_cards.rs:81`, but not with Army of the Damned)
- Sorcery-speed restriction for flashback: NOT TESTED (no timing-restriction test for flashback)
