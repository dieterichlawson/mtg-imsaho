## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
**Type line**: Creature — Spirit Bird
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Mana cost {3}{U} encoded correctly: `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Blue)])` matches oracle — pass
- Card types `[Creature]`, supertypes `[]`, subtypes `["Spirit", "Bird"]` match type line "Creature — Spirit Bird" — pass
- P/T 3/2 matches oracle — pass
- `keywords: vec![Keyword::Flying]` present; `Keyword::Flying` is defined in `types.rs:290` — pass
- `oracle_text: "Flying"` matches oracle text exactly — pass
- Flying enforcement in combat (`combat.rs:619`): `can_block_attacker` checks `has_keyword(attacker_id, Keyword::Flying, registry)` and requires blocker to have Flying or Reach — pass
- `has_keyword` (`state.rs:987`) checks object keywords (for tokens), card registry keywords, continuous-effect grants, conditional keywords, and until-EOT grants — all paths correctly detect Flying — pass
- No triggered abilities, activated abilities, or continuous effects declared — consistent with oracle text (vanilla flyer) — pass
- No flashback cost, additional cost, or dynamic_pt — correct for this card — pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Flying blocks another flyer: `tests/keywords.rs:43` (`flyer_can_block_flyer` uses Moon Heron as blocker) — TESTED
- Flying cannot be blocked by ground creature (Moon Heron as attacker): `tests/keywords.rs:55` (`reach_can_block_flying`) and `tests/keywords.rs:455` (`blocker_validation_rejects_ground_blocking_flyer`) — TESTED
- First strike kills Moon Heron (2 damage = lethal, toughness 2): `tests/keywords.rs:391` (`first_strike_kills_before_normal_damage`) — TESTED
- P/T 3/2 and correct mana cost: NOT TESTED explicitly (covered implicitly by combat damage tests above)
- Spirit and Bird subtypes: NOT TESTED
