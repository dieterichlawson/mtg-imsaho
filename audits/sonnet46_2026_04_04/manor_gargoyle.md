## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Defender\nThis creature has indestructible as long as it has defender.\n{1}: Until end of turn, this creature loses defender and gains flying.
**Type line**: Artifact Creature — Gargoyle
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **"As long as" continuous re-evaluation**: PASS. The indestructible condition is implemented as `ContinuousEffect::ConditionalKeyword { keyword: Keyword::Indestructible, condition: EffectCondition::SelfHasKeyword(Keyword::Defender), scope: EffectScope::OnSelf }`. This is evaluated every time `has_keyword(id, Indestructible)` is called via `has_conditional_keyword` → `check_condition(SelfHasKeyword(Defender))`. It is not snapshotted at ETB — it continuously re-checks whether Defender is present.

- **Defender removal via `until_end_of_turn_removed_keywords`**: PASS. `on_activate_ability` pushes `UntilEndOfTurnKeyword { target: object_id, keyword: Defender }` to `state.until_end_of_turn_removed_keywords`. `has_keyword` checks this list first and returns false if an entry is found, so `has_keyword(id, Defender)` returns false after activation. `check_condition(SelfHasKeyword(Defender))` also checks `until_end_of_turn_removed_keywords` in both the `obj.keywords` branch and the `card_data.keywords` branch, so indestructible is correctly dropped.

- **"As long as" + lethal damage ruling**: PASS. The ruling states "If Manor Gargoyle loses indestructible after having been dealt lethal damage earlier in the turn, it will be destroyed." The SBA loop in `engine.rs:3120-3126` runs after every action via `check_state_based_actions_with_registry`. When the {1} ability fires, indestructible is lost, and the next SBA pass finds lethal damage still marked → calls `try_destroy` → `has_keyword(id, Indestructible, registry)` returns false → creature is destroyed. Correct.

- **End-of-turn cleanup**: PASS. `engine.rs:3021-3025` clears `until_end_of_turn_keywords` and `until_end_of_turn_removed_keywords` at the cleanup step. Both the Flying grant and the Defender removal are correctly undone at end of turn.

- **Defender prevents attacking**: PASS. `combat.rs:eligible_attackers` calls `has_keyword(id, Keyword::Defender, registry)` to filter out Defenders. After activation, this returns false, so Manor Gargoyle correctly becomes eligible to attack. Before activation it is correctly excluded.

- **Activated ability cost {1}, no tap, not once-per-turn**: PASS. `ActivatedAbilityDef` has `cost: ManaCost::new(vec![ManaSymbol::Generic(1)])`, `requires_tap: false`, `once_per_turn: false`. Oracle text specifies `{1}` with no tap symbol and no once-per-turn restriction.

- **Flying gained until end of turn**: PASS. `on_activate_ability` pushes `UntilEndOfTurnKeyword { target: object_id, keyword: Flying }` to `state.until_end_of_turn_keywords`. `has_keyword` checks this list and returns true for Flying. Cleared at cleanup.

- **Mana cost, P/T, types, subtypes**: PASS. `cost: ManaCost::new(vec![ManaSymbol::Generic(5)])` ({5} generic), `power: Some(4), toughness: Some(4)`, `card_types: vec![CardType::Artifact, CardType::Creature]`, `subtypes: vec!["Gargoyle"]`. All match the oracle.

- **Indestructible loss when creature transforms or loses types**: PASS for the implemented cases. The conditional re-evaluates every SBA cycle. Any mechanism that removes Defender (e.g., a further `until_end_of_turn_removed_keywords` entry) would correctly cause indestructible to be lost.

### Test coverage
- **{1} loses Defender, gains Flying (basic activation)**: `tier15_cards.rs:734` — TESTED, but the Defender-loss assertion (`!obj.keywords.contains(&Keyword::Defender)`) is trivially true because `obj.keywords` is always empty for non-token cards created with `named_creature` (keywords live in `card_data`, not in `obj.keywords`). The test should use `!state.has_keyword(gargoyle, Keyword::Defender, &reg)` instead. The Flying grant is tested correctly via `until_end_of_turn_keywords`.
- **Conditional indestructible while having Defender**: NOT TESTED
- **Indestructible lost when Defender removed by activation**: NOT TESTED
- **Lethal damage marked while indestructible → activation → destroyed (the Scryfall ruling)**: NOT TESTED
- **End-of-turn cleanup restores Defender and removes Flying**: NOT TESTED
- **Eligible to attack after activation**: NOT TESTED
