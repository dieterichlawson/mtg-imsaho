## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}{W}, Sacrifice this creature: Exile target enchantment.
**Type line**: Creature — Fox
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Sacrifice before exile — ordering**: The engine pays the sacrifice cost (line 1748: `crate::destruction::sacrifice(&mut new_state, *object_id, registry)`) before calling `on_activate_ability` (line 1802). The fox is in the graveyard when the exile effect fires. The effect targets the enchantment via `targets`, not the fox's current zone, so the exile proceeds correctly. Pass.
- **`on_activate_ability` called after object is sacrificed**: The engine's `behavior_card_id` lookup (lines 1783–1798) calls `b.activated_abilities(&new_state, *object_id, registry)` after the fox is sacrificed. Since `SilverchaseFox::activated_abilities` ignores state and always returns one ability, the condition evaluates true and `behavior_card_id` = fox's card ID. `on_activate_ability` is correctly dispatched. Pass.
- **Target still on battlefield check**: `on_activate_ability` guards with `state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before exiling. If the target leaves the battlefield between cost payment and resolution (not possible in this synchronous engine, but the guard is correct). Pass.
- **`HasCardType(Enchantment)` filter in `generate_ability_targets`**: `matches_target_filter` checks `obj.card_types.contains(t)`. In real gameplay via `setup_game`, `card_types` is populated from `card_data.card_types.clone()` at line 2681. Enchantments correctly have `CardType::Enchantment` in their `obj.card_types`, so the filter works. Pass.
- **No timing restriction (instant speed)**: Code sets `sorcery_speed_only: false`, consistent with the oracle text having no speed restriction. Pass.
- **No tap required, no summoning sickness interference**: Code sets `requires_tap: false`. Newly summoned fox can immediately use this ability. Pass.
- **No once-per-turn restriction**: Code sets `once_per_turn: false`. Consistent with oracle text. Pass.
- **Fox cannot target itself**: Fox has `card_types = [CardType::Creature]` only; the filter requires `CardType::Enchantment`. The fox will never be offered as a valid target for its own ability. Pass.
- **Hexproof enchantments cannot be targeted**: `generate_ability_targets` applies `can_be_targeted` before `matches_target_filter`. Hexproof enchantments controlled by the opponent are excluded. Pass.
- **Mana cost matches**: `{1}{W}` → `Generic(1), Colored(White)` in both spell cost and ability cost. Pass.
- **Card data (name, types, subtypes, P/T, oracle_text)**: All match oracle text exactly. Pass.

### Test coverage
- Basic execution (sacrifice fox, exile enchantment): `tests/tier8_cards.rs:54` — TESTED
- Fox ends in graveyard after ability: `tests/tier8_cards.rs:78` — TESTED
- Target enchantment ends in exile: `tests/tier8_cards.rs:84` — TESTED
- Legal action generation offers enchantments as valid targets in a real game: NOT TESTED (test uses `named_creature` which omits `card_types` and bypasses `generate_ability_targets` by submitting the action directly)
- Ability not offered when no enchantments are on the battlefield: NOT TESTED
- Hexproof enchantment cannot be targeted: NOT TESTED
- Ability usable at instant speed (during opponent's turn): NOT TESTED
- Newly summoned fox can activate immediately (no summoning sickness block): NOT TESTED
