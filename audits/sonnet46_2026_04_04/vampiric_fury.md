## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Vampire creatures you control get +2/+0 and gain first strike until end of turn. (They deal combat damage before creatures without first strike.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Vampire subtype check in `on_resolve` only reads `registry.card_data(obj.card_id)` and never checks `obj.subtypes` — `mtg-engine/src/cards/isd/vampiric_fury.rs:44-46`
  - Oracle text says: `"Vampire creatures you control get +2/+0 and gain first strike until end of turn."`
  - Code does: `registry.card_data(obj.card_id).map(|data| data.subtypes.iter().any(|s| s == "Vampire")).unwrap_or(false)` — this check ignores `obj.subtypes`, which means three concrete in-game cases are silently excluded:
    1. **Vampire tokens** (e.g., the 2/2 black Vampire token created by Bloodline Keeper's `{T}` ability): tokens have `card_id = CardId(0)` (sentinel), so `registry.card_data(CardId(0))` returns `None`, which unwraps to `false`. The "Vampire" type is stored exclusively on `obj.subtypes`, never in the registry.
    2. **Olivia-made Vampires**: Olivia Voldaren's `{1}{R}` ability appends `"Vampire"` to `obj.subtypes` of the damaged creature (`olivia_voldaren.rs:108-109`). After the ability resolves, that creature is a Vampire via `obj.subtypes`, but the registry still shows its original card data with no "Vampire" subtype. Vampiric Fury's filter returns `false` for it.
    3. **Transformed Stalking Vampire** (Screeching Bat back face): `apply_transform` sets `obj.subtypes = ["Vampire"]` and `obj.is_transformed = true` (`helpers.rs:261`), but `card_id` still points to the Screeching Bat entry. `registry.card_data(screeching_bat_id)` returns front-face data with `subtypes = ["Bat"]`, so the filter returns `false` even though the creature is currently Stalking Vampire.

  The correct pattern — checking both `registry.card_data()` AND `obj.subtypes` — is already used elsewhere in the codebase: `state.rs` `matches_filter` (lines 666-672), `bloodline_keeper.rs` `count_vampires` (lines 20-25), and `rakish_heir.rs` (lines 49-51). Vampiric Fury is the only Vampire-tribal card that omits the `obj.subtypes` check.

### Tricky interactions checked

- **Vampire tokens from Bloodline Keeper buffed by Vampiric Fury**: FAIL — tokens have `card_id = CardId(0)`; `registry.card_data` returns `None`; `obj.subtypes` is not checked; tokens receive no bonus.
- **Olivia-made Vampires buffed by Vampiric Fury**: FAIL — Olivia's ability stores the Vampire subtype in `obj.subtypes`, not registry; code's filter misses them.
- **Transformed Stalking Vampire (Screeching Bat) buffed by Vampiric Fury**: FAIL — `registry.card_data` returns Screeching Bat front-face data (subtype "Bat"); `obj.subtypes` (set to `["Vampire"]` by `apply_transform`) is not checked.
- **Snapshot at resolution time (ruling: only Vampires on battlefield when spell resolves get the bonus)**: PASS — code collects `vampire_ids` into a `Vec` before pushing effects, so late-arriving Vampires are correctly excluded.
- **Non-Vampire creature not buffed**: PASS — the registry check (though incomplete) correctly excludes creatures with no Vampire in their static card data.
- **Power modifier is +2/+0, not +2/+2 or any other value**: PASS — `power_mod: 2, toughness_mod: 0` matches oracle.
- **First strike granted (not double strike)**: PASS — `Keyword::FirstStrike` is correct.
- **Until-end-of-turn cleanup**: PASS — `until_end_of_turn_effects` and `until_end_of_turn_keywords` are both cleared during the cleanup step in `engine.rs:3021-3022`.
- **`move_spell_after_resolve` called**: PASS — `state.move_spell_after_resolve(object_id)` at `vampiric_fury.rs:67`; flashback exile is handled correctly.
- **Mana cost {1}{R}**: PASS — `Generic(1), Colored(Red)` matches oracle.
- **Card type Instant**: PASS — `card_types: vec![CardType::Instant]` matches oracle type line.
- **Affects only creatures the controller controls ("you control")**: PASS — filter checks `obj.controller == controller`.

### Test coverage

- Basic Vampire buff (registry Vampire, Markov Patrician): `innistrad_cards.rs:390` — TESTED
- Non-Vampire not buffed: `innistrad_cards.rs:408-410` — TESTED
- Vampire token (Bloodline Keeper token) buffed by Vampiric Fury: NOT TESTED
- Olivia-made Vampire buffed by Vampiric Fury: NOT TESTED
- Transformed Stalking Vampire buffed by Vampiric Fury: NOT TESTED
- Snapshot ruling (creature becomes Vampire after resolution, doesn't get bonus): NOT TESTED
- Effects expire at cleanup: NOT TESTED
