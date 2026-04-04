## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature gets +3/+0. As long as equipped creature is a Human, it has lifelink. Equip {3}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues

- **Snapshot "as long as" — lifelink not re-evaluated when equipped creature transforms** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 14–34 and 84–92)
  - Oracle text says: `As long as equipped creature is a Human, it has lifelink.`
  - Code does: `update_effects` is called exactly once in `on_activate_ability` when the equip ability resolves. It evaluates the Human condition at that moment and writes a static `instance_continuous_effects` vec. There is no mechanism to call `update_effects` again if the equipped creature later transforms (e.g., Gatstaf Shepherd → Gatstaf Howler). The effect is therefore a snapshot, not a continuous re-evaluation. A Human creature equipped with the Cleaver that subsequently transforms into a non-Human Werewolf will retain lifelink it should lose; a non-Human Werewolf that transforms back into a Human will lack lifelink it should gain.

- **Human check ignores transformed state — back-face DFCs incorrectly identified as Human at equip time** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 15–18)
  - Oracle text says: `As long as equipped creature is a Human, it has lifelink.`
  - Code does:
    ```rust
    let is_human = state.get_object(creature_id)
        .and_then(|o| registry.card_data(o.card_id))
        .map(|d| d.subtypes.iter().any(|s| s == "Human"))
        .unwrap_or(false);
    ```
    `registry.card_data(card_id)` always returns the **front-face** `CardData` (it calls `b.card_data()`; see `cards/mod.rs:489–491`). It never checks `o.is_transformed` or calls `behavior.back_face_data()`. If a creature is already in its transformed (back-face) state when the equip resolves — for example, Gatstaf Shepherd transformed into Gatstaf Howler (`subtypes: vec!["Werewolf"]`) — the check still reads the front-face subtypes `["Human", "Werewolf"]` and returns `is_human = true`, incorrectly granting lifelink to a non-Human. The correct pattern (used by `matches_filter` in `state.rs:654–672`) checks `creature.is_transformed` and uses `behavior.back_face_data().subtypes` for DFCs.

- **Human check ignores runtime object subtypes — Human tokens never get lifelink** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 15–18)
  - Oracle text says: `As long as equipped creature is a Human, it has lifelink.`
  - Code does: the same `registry.card_data(o.card_id)` call. Tokens use a sentinel `CardId(0)`; `registry.card_data(CardId(0))` returns `None` (no registry entry for the sentinel), so the chain short-circuits to `unwrap_or(false)`. A Human token (subtypes stored in `o.subtypes`, not the registry) would therefore never be recognized as Human and would never receive lifelink. The correct pattern (used by `matches_filter` in `state.rs:665–672`) checks BOTH `registry.card_data(...).subtypes` AND `creature.subtypes` to cover tokens.

### Tricky interactions checked

- **"As long as" — continuous re-evaluation**: FAIL. `update_effects` is only called in `on_activate_ability`. No engine hook re-calls it when the equipped creature's type changes (e.g., on transform). The lifelink grant/removal is therefore a snapshot, violating the "as long as" continuous-evaluation requirement.
- **Equipped creature transforms Human→Werewolf after equip**: FAIL. Lifelink is not removed when the creature flips to a non-Human back face.
- **Equipped creature transforms Werewolf→Human after equip**: FAIL. Lifelink is not granted when the creature flips back to a Human front face.
- **Equipping to an already-transformed DFC (back-face non-Human)**: FAIL. `registry.card_data()` returns front-face data; a transformed Gatstaf Howler (no Human subtype on back face) is still seen as Human because the front-face Gatstaf Shepherd has "Human" in its subtypes.
- **Human token as equipped creature**: FAIL. Token subtypes live in `o.subtypes` not the registry. `registry.card_data(CardId(0))` returns `None`, so the token is never identified as Human and never receives lifelink.
- **Non-Human creature equipped — no lifelink granted**: PASS. The snapshot at equip time correctly skips the `GrantKeyword` effect for non-Humans.
- **Human creature equipped — lifelink granted at equip time**: PASS. For a non-transformed Human (e.g., Champion of the Parish), the front-face registry data has "Human" and is_human is correctly true.
- **+3/+0 always granted (unconditional)**: PASS. `ModifyPT { power: 3, toughness: 0, scope: EffectScope::Attached }` is always present in the `instance_continuous_effects` vec (both Human and non-Human branches), and `EffectScope::Attached` correctly limits the effect to the attached creature.
- **Equip targets own creatures only**: PASS. `TargetFilter::YouControl` in `ActivatedAbilityDef` and the `is_valid_target` check enforce this.
- **Equip is sorcery speed only**: PASS. `sorcery_speed_only: true` in `ActivatedAbilityDef`.
- **Equip cost is {3}**: PASS. `ManaCost::new(vec![ManaSymbol::Generic(3)])` matches the oracle.
- **Equipment enters unattached and stays on battlefield**: PASS. `on_resolve` calls `state.move_object(object_id, Zone::Battlefield)` and sets `is_equipment = true`.
- **Mana cost {3} (cast)**: PASS. `ManaCost::new(vec![ManaSymbol::Generic(3)])` in `card_data`.
- **Keyword enum — Equip absent**: NOT AN ISSUE. The engine's `Keyword` enum has no `Equip` variant; equip is modeled as an `ActivatedAbilityDef`, not a keyword flag.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Non-Human equipped creature gets +3/+0 but no lifelink: `tier9_equipment.rs:264` (`butchers_cleaver_non_human_gets_power_no_lifelink`) — TESTED
- Human equipped creature gets +3/+0 and lifelink: `tier9_equipment.rs:281` (`butchers_cleaver_human_gets_power_and_lifelink`) — TESTED
- "As long as" re-evaluation after creature transforms Human→non-Human: NOT TESTED
- "As long as" re-evaluation after creature transforms non-Human→Human: NOT TESTED
- Equipping to already-transformed back-face non-Human (wrong-face subtype check): NOT TESTED
- Human token as equipped creature (token subtype check): NOT TESTED
- Card data (mana cost, types, subtypes): `tier9_equipment.rs:253` (`butchers_cleaver_has_correct_data`) — TESTED
