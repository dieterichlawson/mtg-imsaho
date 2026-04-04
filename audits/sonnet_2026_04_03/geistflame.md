## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Geistflame deals 1 damage to any target. Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues
- Engine AnyTarget implementation excludes planeswalkers (engine.rs:1074-1090)
  - Oracle text says: `Geistflame deals 1 damage to any target`
  - Code does: `AnyTarget` only includes creatures (objects with power) and players, but not planeswalkers. In modern MTG post-2018 Dominaria rule change, "any target" includes creatures, players, AND planeswalkers.
- Oracle text field incomplete (geistflame.rs:22)
  - Oracle text says: `Geistflame deals 1 damage to any target. Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`
  - Code does: `oracle_text: "Geistflame deals 1 damage to any target.".into()` (missing flashback reminder text)
- Keywords field missing (geistflame.rs:23)
  - Oracle text says: `Keywords: Flashback`
  - Code does: `keywords: vec![]` (should include `Keyword::Flashback`)

### Tricky interactions checked
- Any target should include planeswalkers: FAIL - engine AnyTarget implementation missing planeswalker support
- Flashback cost correctly set to {3}{R}: PASS - matches oracle text
- Flashback spells are exiled after resolution: PASS - `move_spell_after_resolve` correctly handles flashback via `cast_with_flashback` flag
- "May cast" choice for flashback: PASS - engine properly presents flashback as optional casting choice
- Damage amount (1): PASS - `resolve_damage(state, object_id, targets, 1)` correctly deals 1 damage
- Target requirement AnyTarget: PARTIAL - correctly includes creatures and players but missing planeswalkers

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Flashback offered from graveyard with sufficient mana: `flashback.rs:23` 
- Flashback not offered from hand: `flashback.rs:45`
- Flashback not offered without sufficient mana: `flashback.rs:65`
- Flashback spell exiled after resolution: `flashback.rs:86`
- Normal cast goes to graveyard: `flashback.rs:110`
- Flashback spell countered still exiled: `flashback.rs:129`
- Basic damage dealing to creatures: `tier2_spells.rs:33`
- Flashback spell fizzling goes to exile: `fizzle.rs:137`
- Planeswalker targeting with "any target": NOT TESTED
- Oracle text completeness: NOT TESTED
- Keywords field accuracy: NOT TESTED