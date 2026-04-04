## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, target creature gets -4/-4 until end of turn.
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Intervening-if clause (CR 603.4)**: Oracle says "if a creature died this turn" — the condition must be true both when the trigger event occurs AND when it resolves. The engine's `collect_triggers` always puts ETB triggers on the stack for any registered card (no morbid check at collect time); the check happens inside `on_enter_battlefield` at resolution. This is technically a rules deviation, but is functionally equivalent in this engine because: (a) `creature_died_this_turn` is monotonically set to `true` within a turn and never reverts to `false` until the next turn starts, and (b) the game loop calls `triggers::process_triggers` synchronously after each action, with no player-priority window between the ETB event and trigger resolution. A creature cannot die "in response" to the ETB trigger going on the stack. Result: no observable behavioral difference. PASS.
- **"if no creatures have died, ability won't trigger at all" (ruling 1)**: When `creature_died_this_turn` is false, `on_enter_battlefield` returns immediately before calling `present_target_choice`, so no target is selected and no debuff is applied. PASS.
- **Self-targeting when sole creature (ruling 2)**: Code uses `creature_targets(state)` which collects all battlefield creatures including the banshee itself. When the banshee is the only creature, `present_target_choice` is called with 1 mandatory target and auto-applies the debuff without requiring player input. PASS.
- **Triggers only once (ruling 1)**: A single `TriggeredAbilityDef { kind: TriggerKind::EntersBattlefield }` is declared; one ETB event fires once per entering. PASS.
- **"until end of turn" cleanup**: `until_end_of_turn_effects` is cleared at the cleanup step (`engine.rs:3021`). `effective_toughness` and `effective_power` both iterate `until_end_of_turn_effects` to apply the modifier. PASS.
- **Mandatory targeting (no "you may")**: `present_target_choice` called with `optional: false`. PASS.
- **-4/-4 magnitude**: `DebuffUntilEOT { power: -4, toughness: -4 }` matches the oracle text. PASS.
- **`creature_died_this_turn` correctly set**: Verified set in `sba.rs` (lines 96, 144) for SBA-based deaths and `destruction.rs` (line 100) for sacrifices/destroy effects. Cleared at turn start (`engine.rs:2888`). PASS.
- **"target creature" scope (not opponent-only)**: `creature_targets(state)` returns all battlefield creatures regardless of controller, matching the oracle text which says "target creature" with no restriction. PASS.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Morbid ability fires when creature died this turn: `card_fixes.rs:115` (`morkrut_banshee_can_target_self`)
- Self-targeting when sole creature: `card_fixes.rs:115` (`morkrut_banshee_can_target_self`)
- Morbid ability does NOT fire when no creature died this turn: NOT TESTED (only tested via `process_triggers` synchronously; no explicit test asserting no effect)
- Multiple targets available — player chooses: NOT TESTED
- Until-end-of-turn expiry: NOT TESTED directly for this card (general cleanup tested elsewhere)
- `creature_died_this_turn` set by SBA death: `card_mechanics.rs:28` (`morbid_flag_set_on_creature_death`)
- `creature_died_this_turn` reset on new turn: `card_mechanics.rs:43` (`morbid_flag_resets_on_new_turn`)
- `creature_died_this_turn` set by sacrifice: `card_mechanics.rs:991` (`sacrifice_triggers_morbid`)
