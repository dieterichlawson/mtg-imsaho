## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature has first strike.
As long as equipped creature is a Human, it gets +1/+1.
Equip {1}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues

- "As long as" condition is evaluated once at equip time, not continuously re-evaluated.
  - Oracle text says: `As long as equipped creature is a Human, it gets +1/+1.`
  - Code does: `update_effects` (sharpened_pitchfork.rs:14–34) evaluates `is_human` at the moment of equipping (called only from `on_activate_ability` at line 90) and bakes the result into `instance_continuous_effects`. When the equipped creature later changes its type — e.g., a Human Werewolf (Village Ironsmith, subtypes: `["Human", "Werewolf"]`) transforms into its non-Human back face (Ironfang, subtypes: `["Werewolf"]`) — `update_effects` is never called again. The `ModifyPT { power: 1, toughness: 1, scope: EffectScope::Attached }` entry stays in `instance_continuous_effects`, and `continuous_pt_mods` (state.rs:726–768) reads that entry and applies it as long as the pitchfork remains attached — even though the creature is no longer a Human. The condition must be continuously checked per the oracle's "as long as" wording; the snapshot approach is incorrect. The reverse direction also fails: if the pitchfork is equipped to a non-Human that later transforms into a Human, it will not receive the +1/+1.

- Human subtype check in `update_effects` only reads registry data, not runtime `obj.subtypes`, missing Human tokens.
  - Oracle text says: `As long as equipped creature is a Human, it gets +1/+1.`
  - Code does (sharpened_pitchfork.rs:15–18):
    ```rust
    let is_human = state.get_object(creature_id)
        .and_then(|o| registry.card_data(o.card_id))
        .map(|d| d.subtypes.iter().any(|s| s == "Human"))
        .unwrap_or(false);
    ```
    Tokens have `card_id = CardId(0)` and `registry.card_data(CardId(0))` returns `None`, so the chain short-circuits to `false` even when the token's `obj.subtypes` contains `"Human"`. The correct pattern (used in `check_condition` in state.rs:1086–1093) checks both:
    ```rust
    o.subtypes.iter().any(|s| s == subtype)
    || registry.card_data(o.card_id)
        .map(|d| d.subtypes.iter().any(|s| s == subtype))
        .unwrap_or(false)
    ```
    If a Human token is equipped with the Sharpened Pitchfork, the token will not receive +1/+1 even though it is a Human.

### Tricky interactions checked

- **"As long as" re-evaluation after transform (Human Werewolf → back face)**: FAIL — `update_effects` is only called in `on_activate_ability`; when `VillageIronsmith::on_upkeep` flips `obj.is_transformed` (village_ironsmith.rs:84), the pitchfork's `instance_continuous_effects` is never updated, so the non-Human back face keeps the +1/+1.
- **"As long as" re-evaluation after transform (non-Human → Human)**: FAIL — same root cause; a pitchfork equipped to the non-Human back face will not grant +1/+1 when that creature transforms to its Human front face.
- **Human token subtype check**: FAIL — `registry.card_data(CardId(0))` returns `None` for tokens, so `is_human` is always `false` for Human tokens regardless of `obj.subtypes`.
- **First strike always granted**: PASS — both branches of `update_effects` include `ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached }`, so first strike is unconditional.
- **Re-equip to different creature**: PASS — `on_activate_ability` sets `obj.attached_to = Some(creature_id)` and calls `update_effects` with the new creature, correctly moving the attachment and re-evaluating the Human check for the new target.
- **Equip only targets controller's creatures**: PASS — `activated_abilities` uses `TargetFilter::YouControl` and `is_valid_target` checks `o.controller == caster`.
- **Equip is sorcery-speed**: PASS — `ActivatedAbilityDef.sorcery_speed_only = true`.
- **Equipment detaches when creature dies**: PASS — SBA code in sba.rs:169–188 sets `attached_to = None` when the equipped creature leaves the battlefield; `instance_continuous_effects` becomes stale but the `EffectScope::Attached` check will return `false` (attached_to is None), so effects stop applying correctly.
- **Card data (cost, types, subtypes)**: PASS — cost {2}, types [Artifact], subtypes ["Equipment"] all match oracle.
- **`on_resolve` moves to battlefield and sets `is_equipment = true`**: PASS — correct.

### Test coverage

- "As long as" condition re-evaluated after Human Werewolf transforms: NOT TESTED
- "As long as" condition re-evaluated after non-Human transforms to Human: NOT TESTED
- Human token equip (token subtypes in obj.subtypes): NOT TESTED
- Non-Human gets first strike only, no P/T bonus: `tier9_equipment.rs:215` (`sharpened_pitchfork_non_human_gets_first_strike_only`)
- Human gets first strike and +1/+1: `tier9_equipment.rs:232` (`sharpened_pitchfork_human_gets_first_strike_and_bonus`)
- Card data correct: `tier9_equipment.rs:204` (`sharpened_pitchfork_has_correct_data`)
- Equipment detaches when creature dies: `tier9_equipment.rs:398` (`equipment_detaches_when_creature_dies`, uses Cobbled Wings but same SBA logic)
- Re-equip to different creature: `tier9_equipment.rs:421` (`equipment_can_be_moved_to_different_creature`, uses Cobbled Wings)
