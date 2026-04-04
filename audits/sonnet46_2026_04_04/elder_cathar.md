## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
**Type line**: Creature — Human Soldier
**Status**: ISSUE

### Code issues

- Transformed DFC incorrectly treated as Human in both the single-target auto-select path and the multi-target engine path
  - Oracle text says: `If that creature is a Human, put two +1/+1 counters on it instead.`
  - Code does (`elder_cathar.rs:50-58`, single-target path):
    ```rust
    let is_human = state.get_object(id)
        .map(|o| {
            let obj_has = o.subtypes.iter().any(|s| s == "Human");
            let card_has = registry.card_data(o.card_id)
                .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                .unwrap_or(false);
            obj_has || card_has
        })
        .unwrap_or(false);
    ```
    And (`engine.rs:2219-2227`, multi-target path via `PendingEffect::AddCounters { human_bonus: true }`):
    ```rust
    let is_human = state.get_object(*id)
        .map(|o| {
            let obj_has = o.subtypes.iter().any(|s| s == "Human");
            let card_has = registry.card_data(o.card_id)
                .map(|d| d.subtypes.iter().any(|s| s == "Human"))
                .unwrap_or(false);
            obj_has || card_has
        })
        .unwrap_or(false);
    ```
    Neither check reads `o.is_transformed`. For regular non-token DFCs, `o.subtypes` is empty (subtypes live in the registry), so `obj_has` is always `false`. `card_has` calls `registry.card_data(o.card_id)` which always returns **front-face** data. When a Human Werewolf DFC is transformed (e.g., `Villagers of Estwald` → `Howlpack of Estwald`, front subtypes `["Human", "Werewolf"]`, back subtypes `["Werewolf"]`), `card_has` returns `true` because the front-face registry data still lists "Human". The creature is therefore incorrectly granted 2 counters instead of 1. The correct check is the pattern used in `state.rs:matches_filter` (for `HasSubtype`), which gates on `creature.is_transformed` and consults `back_face_data()` when `true`.

### Tricky interactions checked

- **Single creature auto-select (no player choice)**: PASS — with exactly one valid battlefield creature, the code applies counters directly without presenting a choice. The result is correct for the one-target case, and the Human/non-Human counter count is applied correctly (bugs above aside).
- **Multiple creatures (player chooses target)**: PASS (logic correct) — `ChooseTarget` is presented with `optional: false` matching the mandatory "target" wording, and resolves via `apply_pending_effect` which re-evaluates Human status at resolution time.
- **No valid targets (empty battlefield)**: PASS — the early-return branch when `targets.is_empty()` produces the correct game behavior (ability has no legal targets; resolves without effect).
- **Transformed Human Werewolf as target**: ISSUE — as described above, a transformed creature whose back face lacks "Human" is incorrectly treated as Human, producing 2 counters instead of 1.
- **Human token as target**: PASS — tokens store subtypes in `o.subtypes` directly; `obj_has` correctly finds "Human" in that field.
- **Elder Cathar targeting itself**: PASS — Elder Cathar is in the graveyard when the trigger resolves, excluded by `o.zone == Zone::Battlefield`.
- **"instead" semantics**: PASS — the Human branch gives exactly 2 counters, not 1 + 2; the `count * 2` formula applied to the base of 1 yields 2, matching "instead of 1".
- **Controller lookup after death**: PASS — the `on_dies` handler reads controller from the graveyard object (`state.get_object(object_id)`); controller data is preserved through zone changes, so the correct player's creatures are filtered.
- **Trigger fires unconditionally on death**: PASS — `collect_triggers` in `triggers.rs` pushes a `SelfDies` trigger for any registered card that dies (lines 401–415), without requiring a non-empty description.
- **"target" optionality**: PASS — in the multi-creature path, `optional: false` is passed to `ChooseTarget`, correctly reflecting that the ability's target is mandatory (oracle does not say "may").

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Basic counter grant on death (non-Human target): `tests/tier3_cards.rs:404` (`elder_cathar_grants_counter_on_death`)
- Human target receives 2 counters: `tests/card_mechanics.rs:412` (`elder_cathar_gives_two_counters_to_human`)
- Non-Human target receives 1 counter: `tests/card_mechanics.rs:432` (`elder_cathar_gives_one_counter_to_non_human`)
- Transformed Human Werewolf as target (back face = non-Human): NOT TESTED
- Human token as target (subtypes in `o.subtypes`): NOT TESTED
- Multiple creatures on battlefield (player choice presented): NOT TESTED
- No valid targets (trigger resolves with no effect): NOT TESTED
