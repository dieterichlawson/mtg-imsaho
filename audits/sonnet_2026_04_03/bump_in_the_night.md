## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target opponent loses 3 life.
Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues
- Oracle text field incomplete in implementation: mtg-engine/src/cards/isd/bump_in_the_night.rs:23
  - Oracle text says: `Target opponent loses 3 life.\nFlashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`
  - Code does: `oracle_text: "Target opponent loses 3 life.".into(),` (missing flashback text)

### Tricky interactions checked
- Target opponent restriction: PASS - `is_valid_target` correctly checks `*pid != caster` to exclude self
- Life loss of exactly 3: PASS - `on_resolve` subtracts 3 from target player's life and generates LifeChanged event  
- Flashback cost {5}{R}: PASS - `flashback_cost` field correctly set to 5 generic + 1 red mana
- Flashback exile: PASS - calls `move_spell_after_resolve` which handles exile via `cast_with_flashback` flag
- Sorcery timing restrictions: PASS - handled automatically by engine for sorcery card type
- Player targeting: PASS - `target_requirement` returns `PlayerOnly` as expected

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Normal casting loses 3 life: `mtg-engine/tests/tier2_spells.rs:21` (bump_in_the_night_drains_3)
- Flashback costs {5}{R} and exiles spell: `mtg-engine/tests/flashback.rs:471` (bump_in_the_night_flashback_exiles)
- Target opponent restriction: NOT TESTED
- Sorcery timing restrictions: NOT TESTED
- LifeChanged event generation: NOT TESTED