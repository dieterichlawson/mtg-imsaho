## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying, first strike
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flying blocking restriction: `can_block_attacker` in `combat.rs:619` checks `has_keyword(attacker_id, Keyword::Flying, registry)` and returns false if blocker lacks Flying or Reach — pass
- First strike damage step ordering: `deal_combat_damage` in `combat.rs:136-153` checks for any first/double striker and runs a separate first-strike damage step followed by SBAs before the normal damage step — pass
- `has_keyword` reads from `card_data().keywords` (line 1012 in `state.rs`): Voiceless Spirit's `keywords: vec![Keyword::Flying, Keyword::FirstStrike]` is correctly picked up — pass
- Docstring comment (line 4 of implementation) says `Flying.` only, omitting first strike: this is a documentation inaccuracy but does not affect behavior since the functional `keywords` vec includes both `Keyword::Flying` and `Keyword::FirstStrike` — pass (no behavioral bug)
- No triggered or activated abilities, no continuous effects, no ETB effects: no engine trigger paths to trace — pass (nothing to check)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Both Flying and FirstStrike keywords present on card: `tests/innistrad_cards.rs:106` (`voiceless_spirit_has_flying_and_first_strike`)
- First strike kills blocker before normal damage step (combat behavior): `tests/keywords.rs:383` (`first_strike_kills_before_normal_damage`, uses Voiceless Spirit as the attacker)
- Flying blocking restriction (non-flyer cannot block): NOT TESTED specifically for Voiceless Spirit (general flying blocking tests exist elsewhere in the test suite)
