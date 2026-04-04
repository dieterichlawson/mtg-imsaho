## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying\n{T}: Create a 2/2 black Vampire creature token with flying.\n{B}: Transform this creature. Activate only if you control five or more Vampires.\n\n--- Back Face (Lord of Lineage) ---\nFlying\nOther Vampire creatures you control get +2/+2.\n{T}: Create a 2/2 black Vampire creature token with flying.
**Type line**: Creature — Vampire // Creature — Vampire
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **5+ Vampire count includes Bloodline Keeper itself**: `count_vampires` iterates all objects in `Zone::Battlefield` for the controller, checks `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Vampire"))`, which returns true for Bloodline Keeper's front face. Bloodline Keeper correctly counts itself. PASS

- **Transform gated to front face only**: `activated_abilities` checks `!is_transformed` before adding ability_index 1. When transformed (Lord of Lineage), the {B} transform ability is not offered. PASS

- **Token Vampire subtype for +2/+2 effect**: `create_token_with_subtypes` sets `subtypes: vec!["Vampire".into()]` on the token object. `matches_filter` with `HasSubtype("Vampire")` falls through to `creature.subtypes.iter().any(|s| s == subtype)` for tokens (CardId(0) has no registry entry). Tokens are correctly buffed by Lord of Lineage. PASS

- **Lord of Lineage does NOT buff itself ("Other")**: Scope is `EffectScope::GlobalOther(...)`. In `effect_applies_to`, `GlobalOther` checks `creature_id != source_id` first, excluding Lord of Lineage from its own +2/+2. PASS

- **Continuous effect reads back_face_data when transformed**: In `continuous_pt_mods` (state.rs line 746–749), when `source.is_transformed`, the engine uses `behavior.back_face_data().map(|d| d.continuous_effects)` instead of `card_data().continuous_effects`. The front face has `continuous_effects: vec![]`; the back face has the ModifyPT +2/+2. The effect is only active when transformed. PASS

- **Flying on both faces via registry**: `has_keyword` checks `obj.keywords` first (empty for non-token), then falls through to registry: front face `card_data().keywords = vec![Keyword::Flying]`, back face `back_face_data().keywords = vec![Keyword::Flying]`. Both faces correctly have Flying. PASS

- **Lord of Lineage P/T via dynamic_pt**: `dynamic_pt` returns `Some((5, 5))` when `obj.is_transformed == true`. `effective_power`/`effective_toughness` use this override instead of `obj.power` (still 3 after transform). Back face P/T is 5/5. PASS

- **Token {T} ability available on back face**: ability_index 0 is added unconditionally (no `is_transformed` guard) in `activated_abilities`. Both faces offer the token-creation ability. PASS

- **Sorcery-speed restriction on {B} ability**: Oracle has no sorcery-speed restriction; code sets `sorcery_speed_only: false`. The transform can be activated at instant speed. PASS

- **{B} once-per-turn tracking**: Oracle has no once-per-turn restriction. Code sets `once_per_turn: false`. After transformation, ability_index 1 is removed from `activated_abilities` (via `!is_transformed` guard), so repeated activation of the transform is structurally impossible within a game. PASS

- **"Activate only if" condition re-check at execution time**: The engine re-calls `activated_abilities` on `new_state` at the point of executing `Action::ActivateAbility`. Since activated abilities in this engine resolve immediately (no interactive stack), `new_state` is an unmodified clone of the state from when the action was generated. The vampire count is identical, so the ability is always found. PASS

- **Vampire token HasSubtype check for the count**: `count_vampires` checks `o.subtypes.iter().any(|s| s == "Vampire")` for tokens (which have subtypes set directly on the object) and `registry.card_data(o.card_id).map(|d| d.subtypes...)` for real cards. Both paths work correctly. PASS

- **Transformed DFC subtype in count_vampires**: `count_vampires` uses `registry.card_data(o.card_id)` which returns front face data regardless of `is_transformed`. Both faces are Vampires, so counting Lord of Lineage as a Vampire is correct (the front face data already has "Vampire"). PASS

### Test coverage

- **{T}: Creates a 2/2 black Vampire creature token with flying**: `tier15_cards.rs:1428` (`bloodline_keeper_creates_vampire_token`) — tests token creation but calls `on_activate_ability` directly (bypasses tap-cost check and legal_actions). Token keyword (Flying) and color (black) are NOT asserted.
- **{B}: Transform ability (ability_index 1)**: NOT TESTED
- **"Activate only if you control five or more Vampires" restriction**: NOT TESTED
- **Lord of Lineage +2/+2 to other Vampires**: NOT TESTED
- **Lord of Lineage {T} token ability post-transform**: NOT TESTED
- **Lord of Lineage 5/5 P/T via dynamic_pt**: NOT TESTED
- **Flying on Vampire tokens (keyword check)**: NOT TESTED
- **Lord of Lineage Flying keyword**: NOT TESTED
