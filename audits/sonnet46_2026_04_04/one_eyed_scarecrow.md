## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Defender
Creatures with flying your opponents control get -1/-0.
**Type line**: Artifact Creature — Scarecrow
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Defender prevents attacking**: `combat.rs:579` checks `!state.has_keyword(o.id, Keyword::Defender, registry)` in `eligible_attackers`; Scarecrow is correctly excluded from attacker list. Pass.
- **Flying detection for tokens**: `has_keyword` (state.rs:987) checks `obj.keywords.contains(&keyword)` first, so tokens with `Keyword::Flying` in their object-level keywords are correctly identified. Pass.
- **Flying detection via aura grant** (e.g., Spectral Flight): `has_keyword` checks `GrantKeyword` continuous effects at step 2 (state.rs:1018-1026), so creatures that gained flying from an aura are also caught by the filter. Pass.
- **"As long as" / continuous re-evaluation**: The effect is declared as a `ContinuousEffect` in `card_data()`, not a snapshot. `continuous_pt_mods` is called every time `effective_power` is queried, so if a creature gains or loses flying mid-game, the debuff updates automatically. Pass.
- **EffectScope::Global vs. GlobalOther**: Oracle says "Creatures with flying your opponents control" — the source Scarecrow doesn't have flying and isn't controlled by an opponent, so `Global` (includes self in scan) is functionally identical to `GlobalOther` here. The `And([Opponents, HasKeyword(Flying)])` filter would return false for the Scarecrow itself anyway. Pass.
- **-1/-0 representation**: Oracle says `-1/-0`. Code uses `power: -1, toughness: 0`, where `toughness: 0` means no toughness modification, matching `-0`. Pass.
- **Multiple Scarecrows stacking**: Each Scarecrow source is iterated independently in `continuous_pt_mods`; two Scarecrows would each apply -1/-0 for a total of -2/-0 on opponent flyers. Per MTG rules, identical static effects from multiple sources stack. Pass.
- **Flying temporarily removed** (e.g., until-EOT keyword removal): `has_keyword` checks `until_end_of_turn_removed_keywords` first (state.rs:994-997) before returning true for any keyword, so a flyer that loses flying (e.g., via some effect) would no longer be debuffed. Pass.
- **Flying temporarily granted** (e.g., until-EOT grant): `has_keyword` also checks `until_end_of_turn_keywords` (state.rs:1036-1040), so a creature that temporarily gains flying would be debuffed while it has flying. Pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Opponent's flyer gets -1 power from Scarecrow: `card_mechanics.rs:372` (`one_eyed_scarecrow_debuffs_opponent_flyers`) — TESTED
- Toughness unchanged (-0): `card_mechanics.rs:392` — TESTED
- Opponent's non-flyer not affected: `card_mechanics.rs:395-397` — TESTED
- Own flyer (token with Flying) not debuffed: `card_mechanics.rs:400-403` — TESTED
- Scarecrow as artifact creature can block intimidate creatures: `keywords.rs:228-240` — TESTED
- Defender prevents Scarecrow from attacking: NOT TESTED (no dedicated test, though the combat module logic is covered generically)
- Flying gained via aura (GrantKeyword effect) detected by filter: NOT TESTED for Scarecrow specifically
- Multiple Scarecrows stacking: NOT TESTED
- Creature losing flying mid-game updates debuff: NOT TESTED
