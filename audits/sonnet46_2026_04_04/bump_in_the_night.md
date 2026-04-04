## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target opponent loses 3 life.
Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- LLM player card knowledge is missing flashback ability (`mtg-player/src/llm.rs` line 84)
  - Oracle text says: `Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`
  - Code does: `"- Bump in the Night ({B} sorcery): Target opponent loses 3 life."` — no flashback cost or graveyard-cast information listed. Every other flashback card in the same file (Think Twice, Dream Twist, Travel Preparations, etc.) includes ", flashback {cost}" and a "Can cast from graveyard!" note. Bump in the Night is listed with no flashback information, so the LLM player will never plan mana to cast it from the graveyard and may not recognize the flashback option.

- `oracle_text` field in `card_data()` is incomplete (`mtg-engine/src/cards/isd/bump_in_the_night.rs` line 23)
  - Oracle text says: `Target opponent loses 3 life.\nFlashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`
  - Code does: `oracle_text: "Target opponent loses 3 life.".into()` — the flashback reminder line is absent. This field is rendered verbatim to human players via the CLI card display (`mtg-player/src/cli.rs` lines 702–724). The human player sees no flashback reminder in the card text. (The flashback cost IS separately shown via the `flashback_cost` field at line 728, so this is display-only and does not affect engine mechanics.)

### Tricky interactions checked

- **Flashback exile on normal resolution**: `on_resolve` calls `state.move_spell_after_resolve(object_id)`, which checks `cast_with_flashback` and routes to `Zone::Exile`. Engine also calls `move_spell_after_resolve` after `on_resolve` if the object is still on the stack (`stack.rs` lines 107–111), but since `on_resolve` already moved it, the zone check prevents a double-move. Correct.
- **Flashback exile when countered**: All counterspell implementations in the codebase (`Counterspell`, `Dissipate`, `Lost in the Mist`, Frightful Delusion via PayOrNot) call `state.move_spell_after_resolve(target_id)` rather than `move_object(Zone::Graveyard)`, so a flashback-cast Bump in the Night that is countered is correctly exiled. Correct per ruling: "A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way."
- **Flashback exile when fizzled**: `stack.rs` calls `state.move_spell_after_resolve(object_id)` on fizzle (all targets illegal). Correct.
- **Sorcery timing restriction for flashback**: The graveyard-cast loop in `engine.rs` (lines 692–706) applies the same `is_sorcery_speed` guard (`main phase, stack empty, active player's turn`) to sorcery-type spells. A flashback cast of Bump in the Night can only be initiated during the caster's main phase with an empty stack. Correct per ruling.
- **"Target opponent" restriction (not self)**: `is_valid_target` checks `*pid != caster` and returns `false` for the caster themselves. Combined with `TargetRequirement::PlayerOnly` in the engine which calls both `can_target_player` (hexproof) and `behavior.is_valid_target`, the caster cannot target themselves. Correct.
- **Hexproof player cannot be targeted**: `can_target_player` in `engine.rs` (line 773) checks `state.player_has_hexproof` before allowing a target; Witchbane Orb test confirms this blocks Bump in the Night from targeting a hexproof player. Correct.
- **Hexproof gained after targeting does not fizzle**: `is_target_legal` in `stack.rs` returns `true` unconditionally for `Target::Player`. If the target gains hexproof after Bump in the Night is already on the stack, the spell still resolves. This is correct per MTG rule 115.5 (hexproof prevents targeting, not resolution of already-targeted spells).
- **Life loss vs. damage**: Oracle says "loses 3 life", not "deals 3 damage". Code directly subtracts 3 from life total and emits `GameEvent::LifeChanged` without any damage event (`NonCombatDamageDealt`). This correctly bypasses damage prevention effects and does not trigger lifelink. Correct.
- **State-based action after life loss**: `sba.rs` (lines 22–35) checks `life <= 0` and marks the player as lost. Called after every action via `check_state_based_actions_with_registry`. Correct.
- **Mana cost**: Code declares `ManaSymbol::Colored(Color::Black)` for {B}. Correct.
- **Flashback cost**: Code declares `ManaSymbol::Generic(5), ManaSymbol::Colored(Color::Red)` for {5}{R}. Correct.

### Test coverage

- Basic life loss (Bump in the Night from hand deals 3 life loss to opponent): `tests/tier2_spells.rs:21` — TESTED
- Flashback exiles the card and deals 3 life loss from graveyard: `tests/flashback.rs:471` — TESTED
- Cannot target hexproof player (Witchbane Orb blocks targeting): `tests/witchbane_orb.rs:33` — TESTED
- Cannot target self: NOT TESTED (no test asserts that casting Bump in the Night cannot target P0 when P0 is the caster)
- Countered flashback spell is exiled (not graveyard): NOT TESTED for Bump specifically (covered generically by Dissipate test, but not for flashback target)
- Sorcery timing restriction for flashback (can't cast during opponent's turn or with stack non-empty): NOT TESTED
- LLM player knowledge includes flashback information: NOT TESTED (and the knowledge is wrong)
