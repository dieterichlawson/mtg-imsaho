## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This land enters tapped unless you control a Forest or an Island.
{T}: Add {G} or {U}.
**Type line**: Land
**Status**: ISSUE

### Code issues

- `controller_has_matching_land` only checks object-level subtypes (`o.subtypes`), missing registry-stored subtypes for regularly-played Forest/Island cards
  - File: `mtg-engine/src/cards/isd/hinterland_harbor.rs`, lines 17–23
  - Oracle text says: `"This land enters tapped unless you control a Forest or an Island."`
  - Code does: `o.subtypes.iter().any(|s| s == "Forest") || o.subtypes.iter().any(|s| s == "Island")` — only checks `obj.subtypes`, which is `Vec::new()` for all regular (non-token) cards. For regular cards, subtypes live exclusively in `CardData.subtypes` (accessed via the registry). When a Forest or Island is played through the normal gameplay path (`Action::PlayLand` → `move_object`), `obj.subtypes` is never populated from the registry. As a result, `controller_has_matching_land` returns `false` for any regularly-played Forest or Island, and Hinterland Harbor always enters tapped even when the condition is satisfied.

  Supporting evidence:
  - `state.rs` line 1205: `/// Subtypes on this object (for tokens — regular cards use CardData.subtypes via registry).`
  - `engine.rs` `setup_game` (lines 2670–2682): copies `colors`, `name`, `keywords`, `card_types` to the object but never copies `card_data.subtypes` to `obj.subtypes`.
  - `forest.rs` line 15: `subtypes: vec!["Forest".into()]` — this lives in `CardData`, not the `GameObject`.
  - By contrast, `check_condition` in `state.rs` (lines 1084–1093) correctly checks both `o.subtypes` AND `registry.card_data(o.card_id).subtypes` when testing for subtype control.
  - The existing test `clifftop_retreat_enters_untapped_with_mountain` (same pattern) masks this bug by manually setting `state.get_object_mut(mtn).unwrap().subtypes = vec!["Mountain".into()]` — which is not what happens in real gameplay.

### Tricky interactions checked

- **"you control" scoping**: Checks `objects_in_zone(Zone::Battlefield, controller)` where `controller` is the controller of Hinterland Harbor — correctly ignores lands controlled by opponents: pass (logic correct, but subtype lookup broken as noted above)
- **Self-exclusion**: The `o.id != object_id` guard correctly prevents Hinterland Harbor from counting itself (it has no Forest/Island subtype anyway): pass
- **Forest or Island basic land type (not name)**: Subtype check ("Forest"/"Island") is the correct way to detect any land with those basic types, not just cards named "Forest"/"Island": pass (approach correct, execution broken for registry cards)
- **Mana ability gating on tapped state**: `mana_abilities` returns empty vec when `obj.tapped` — correctly prevents tapping a tapped land for mana: pass
- **Produces {G} or {U}**: `ManaType::Green` and `ManaType::Blue` match the oracle's `{G}` and `{U}`: pass
- **ETB trigger path**: `on_enter_battlefield` is dispatched via `PendingTrigger::EnteredBattlefield`; the trigger resolves only if the object is still on the battlefield (triggers.rs line 895). Correct for a land (lands don't normally bounce between ETB trigger creation and resolution): pass
- **Mana ability deduplication**: Engine deduplicates mana abilities by `(card_id, ability_index)`; Harbor uses `ability_index: 0` for Green and `ability_index: 1` for Blue, so both are offered as distinct choices: pass
- **Token lands with Forest/Island subtype**: Token objects store subtypes directly on `obj.subtypes`, so `controller_has_matching_land` would correctly detect a Forest/Island token land. Only registry-based cards are broken: pass (tokens work, regular cards don't)

### Test coverage

- ETB tapped when no matching land: NOT TESTED (only a `card_data` test exists for Hinterland Harbor)
- ETB untapped with Forest: NOT TESTED
- ETB untapped with Island: NOT TESTED
- Mana ability produces {G}: NOT TESTED directly (only card_data type check tested)
- Mana ability produces {U}: NOT TESTED directly
- Subtype lookup against registry (the broken code path): NOT TESTED — the one analogous test for Clifftop Retreat (`clifftop_retreat_enters_untapped_with_mountain`) manually injects `obj.subtypes`, bypassing the real gameplay code path and masking the bug
