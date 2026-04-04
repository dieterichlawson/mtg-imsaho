## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: 
Front Face - Hanweir Watchkeep:
Defender
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.

Back Face - Bane of Hanweir:
This creature attacks each combat if able.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Type line**: 
Front: Creature — Human Warrior Werewolf
Back: Creature — Werewolf
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Each upkeep" trigger timing**: The upkeep trigger fires on every player's upkeep, not just the controller's. Verified in triggers.rs line 599 where Step::Upkeep triggers are dispatched for all battlefield permanents.
- **"No spells were cast last turn" vs "a player cast two or more spells"**: Front face transform checks `total_spells_last_turn == 0` (sum across all players). Back face transform checks `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (any single player cast 2+). This correctly matches the oracle text difference.
- **First turn protection**: Transform logic includes `!state.is_first_turn` check to prevent transformation on the first turn when there is no "last turn" to evaluate.
- **Defender vs ForceAttack interaction**: Front face has Defender keyword preventing attacks. Back face has `ContinuousEffect::ForceAttack` but no Defender. Engine code correctly prevents forced attacks when Defender is present (engine.rs line 1834).
- **Transform preserves permanent identity**: The same ObjectId is used throughout, only `is_transformed`, `name` and dynamic stats change. The `on_upkeep` method correctly handles both transform directions using the same logic.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic transform mechanics: `hanweir_watchkeep_loses_defender_gains_force_attack` in werewolf_cards.rs:174
- Defender keyword presence/absence: `hanweir_watchkeep_loses_defender_gains_force_attack` in werewolf_cards.rs:174 
- ForceAttack continuous effect on back face: `hanweir_watchkeep_loses_defender_gains_force_attack` in werewolf_cards.rs:174
- Power/toughness changes (1/5 to 5/5): `hanweir_watchkeep_loses_defender_gains_force_attack` in werewolf_cards.rs:174
- First turn transform suppression: NOT TESTED (covered by shared werewolf logic tests for other cards)
- "Any player cast 2+ spells" back-transform condition: NOT TESTED
- Multiple upkeep triggers on different players' turns: NOT TESTED