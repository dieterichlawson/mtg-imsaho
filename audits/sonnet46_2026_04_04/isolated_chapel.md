## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This land enters tapped unless you control a Plains or a Swamp.
{T}: Add {W} or {B}.
**Type line**: Land
**Status**: ISSUE

### Code issues

- Subtype check in `controller_has_matching_land` only reads `obj.subtypes` on game objects, missing subtypes stored in the registry for regular (non-token) cards
  - Oracle text says: `This land enters tapped unless you control a Plains or a Swamp.`
  - Code does: `o.subtypes.iter().any(|s| s == "Plains") || o.subtypes.iter().any(|s| s == "Swamp")` (`mtg-engine/src/cards/isd/isolated_chapel.rs` lines 21–23) — this only checks the runtime `obj.subtypes` field, which is **never populated for regular deck cards** in real gameplay. `setup_game` in `engine.rs` (lines 2669–2683) populates `obj.name`, `obj.colors`, `obj.keywords`, and `obj.card_types` from card data, but does NOT populate `obj.subtypes`. The `PlayLand` handler (engine.rs line 1460–1477) likewise does not populate subtypes. As a result, a Plains or Swamp played from a deck will have `obj.subtypes = []`, the check always returns false, and Isolated Chapel always enters tapped even when you control a Plains or Swamp. By contrast, the `check_condition` helper in `state.rs` (lines 1086–1092) correctly checks both `o.subtypes` AND `registry.card_data(o.card_id).subtypes` to cover both tokens and registry-backed cards. The card's `on_enter_battlefield` receives `_registry` (prefixed with underscore, unused), and `controller_has_matching_land` takes no registry parameter at all, making the fix require adding registry access to the helper.

### Tricky interactions checked

- "Unless you control a Plains or a Swamp" subtype check for registry-backed basic lands: FAIL — `obj.subtypes` is never populated for non-token cards; Plains/Swamp subtypes only exist in the registry, which is not consulted
- "Unless you control a Plains or a Swamp" subtype check for tokens with Plains/Swamp subtype: PASS — tokens have subtypes set directly on `obj.subtypes` at creation, so those would be found
- Self-exclusion (`o.id != object_id`) correctly prevents Isolated Chapel from counting itself as a matching land: PASS
- "You control" — check uses `objects_in_zone(Zone::Battlefield, controller)` which filters by controller, not owner: PASS
- ETB trigger fires correctly for lands: PASS — `collect_triggers` in `triggers.rs` line 351 collects `EnteredBattlefield` for all registered cards (`registry.get(card_id).is_some()`), which includes Isolated Chapel
- ETB resolves via `on_enter_battlefield` with zone check: PASS — triggers.rs line 895 verifies the object is still on the battlefield before calling the handler
- Mana ability ({T}: Add {W} or {B}) only available when untapped and on battlefield: PASS — `mana_abilities` checks `obj.zone == Zone::Battlefield && !obj.tapped`
- Mana ability produces correct colors (White and Black): PASS — `ManaType::White` and `ManaType::Black` match the oracle text {W} and {B}
- Mana ability requires tap (`requires_tap: true`): PASS
- Oracle text field content: minor textual difference only — code says "Isolated Chapel enters the battlefield tapped unless..." whereas Scryfall says "This land enters tapped unless..."; the `oracle_text` field is display-only and does not drive any engine behavior, so no gameplay impact

### Test coverage

- Card data type check (`isolated_chapel_card_data` in `innistrad_simple_cards.rs:99`): tests only that `card_types` contains `CardType::Land` — NOT a meaningful behavioral test
- Isolated Chapel enters tapped without matching land: NOT TESTED
- Isolated Chapel enters untapped with a Plains: NOT TESTED (note: analogous tests for Clifftop Retreat and Woodland Cemetery exist but work around the bug by manually setting `obj.subtypes` on the test object, e.g. `woodland_cemetery_enters_untapped_with_swamp` line 131: `state.get_object_mut(swp).unwrap().subtypes = vec!["Swamp".into()];` — these tests pass with the buggy code because they supply what real gameplay never provides)
- Isolated Chapel enters untapped with a Swamp: NOT TESTED
- Mana ability produces {W}: NOT TESTED
- Mana ability produces {B}: NOT TESTED
- Tapped chapel produces no mana abilities: NOT TESTED
