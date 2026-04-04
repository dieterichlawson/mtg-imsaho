## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- `build_cast_target_spec` returns `CastTargetSpec::SingleTarget` for modal spells containing a `TwoTargets` inner mode, blocking interactive players from choosing mode 2
  - Oracle text says: `• Return two target Zombie cards from your graveyard to your hand.`
  - Code does: In `engine.rs:1212-1219`, `build_cast_target_spec` for `ModalChoice` iterates over each mode and calls `valid_targets_for_req` on it. For mode 2 (`TwoTargets(GraveyardCreatureOfSubtype("Zombie"), GraveyardCreatureOfSubtype("Zombie"))`), `valid_targets_for_req` hits the `_ => vec![]` fallthrough at `engine.rs:1185` because `TwoTargets` is not handled there. This means `all_options` receives only mode 1 (single-creature) targets and the result is `CastTargetSpec::SingleTarget(mode1_targets_only)`. Both the CLI player (`cli.rs:832-838`: chooses one target from `SingleTarget` options) and the LLM player (`llm.rs:512-517`: chooses one target from `SingleTarget` options) can therefore only ever choose one target, making mode 2 entirely inaccessible through those interfaces. The random/bot player is unaffected because it uses `generate_cast_actions_with_targets` (`engine.rs:979-984`) which correctly recurses into `TwoTargets` for mode 2.

### Tricky interactions checked

- **Mode 1 "your graveyard" restriction**: `GraveyardCreature` in `valid_targets_for_req` scans all graveyards, but `is_valid_target` (card file line 50) checks `o.owner == caster`. Combined, only the caster's creature cards are targetable. Pass.
- **Mode 2 Zombie creature check**: `GraveyardCreatureOfSubtype("Zombie")` in `valid_targets_for_req` (`engine.rs:1142-1159`) checks both `o.subtypes` and `registry.card_data(...).subtypes`, covering registry cards and tokens. Pass.
- **Mode 2 same-target guard**: `generate_cast_actions_with_targets` for `TwoTargets` (`engine.rs:993`) checks `if t1 != t2` before generating a pair, preventing targeting the same Zombie card twice. Pass.
- **Fizzle (all targets illegal at resolution)**: `stack.rs:79-86` fizzles if `!targets.iter().any(|t| is_target_legal(...))`. For `ModalChoice`, `is_target_legal` at `stack.rs:12-14` returns true if the target is legal under any mode. Mode 0 (`GraveyardCreature`) accepts any graveyard object, so Zombie targets are covered even if `chosen_mode` detection is wrong. Pass.
- **Partial-target fizzle (one of two mode-2 targets becomes illegal)**: If one of two Zombie targets leaves the graveyard in response, `any_legal` is still true. `on_resolve` at card line 61 re-checks `obj.zone == Zone::Graveyard` before moving, so only the still-legal card is returned. Pass.
- **`move_spell_after_resolve` present**: `on_resolve` calls `state.move_spell_after_resolve(object_id)` at line 70. Pass.
- **No erroneous `chosen_mode` effect on resolution**: `on_resolve` does not consult `obj.chosen_mode` at all; it just iterates `targets`. The mode detection bug in `detect_modal_choice_mode` (assigns mode 0 when mode 1 is intended, because Zombie creature cards also satisfy `GraveyardCreature`) has no observable effect on card behavior. Pass.
- **"your graveyard" — mode 2 scope**: Same as mode 1; `is_valid_target` owner check applies to all `GraveyardCreatureOfSubtype` candidates. Pass.
- **Duplicate actions for mode 2**: `generate_cast_actions_with_targets` for `TwoTargets` generates `[Z1, Z2]` and `[Z2, Z1]` as separate actions. Both are legal and produce identical game outcomes. This is a cosmetic redundancy, not a behavioral error. Pass.
- **Card data (cost, types, oracle_text)**: `{B}` / Sorcery / no supertypes / no subtypes / oracle_text matches. Pass.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Mode 1 returns single creature to hand: `tests/ghoulcallers_chant.rs:22` and `tests/tier11_cards.rs:105` — TESTED
- Mode 2 returns two Zombies to hand: `tests/ghoulcallers_chant.rs:38` and `tests/tier11_cards.rs:120` — TESTED
- Mode 1 legal actions include single-creature targets: `tests/ghoulcallers_chant.rs:61` — TESTED
- Mode 2 legal actions include two-Zombie pairs (via `generate_cast_actions_with_targets`): `tests/ghoulcallers_chant.rs:89` — TESTED (bot path only; interactive-UI path untested)
- Mode 2 not available for non-Zombie creatures: `tests/ghoulcallers_chant.rs:118` — TESTED
- Cannot target opponent's graveyard: `tests/ghoulcallers_chant.rs:160` — TESTED
- Mixed graveyard (one Zombie, one non-Zombie): `tests/ghoulcallers_chant.rs:182` — TESTED
- `build_cast_target_spec` for ModalChoice+TwoTargets returns correct `CastTargetSpec` for interactive players: NOT TESTED
- Mode 2 accessible through CLI/LLM player interface: NOT TESTED
- Fizzle when all targets leave graveyard in response: NOT TESTED
- Partial-target scenario (one of two mode-2 targets becomes illegal): NOT TESTED
