## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This land enters tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
**Type line**: Land
**Status**: ISSUE

### Code issues
- `controller_has_matching_land` only checks runtime object subtypes (`o.subtypes`), never the registry — so real Mountain and Plains cards are never detected, causing Clifftop Retreat to always enter tapped when it should enter untapped (`mtg-engine/src/cards/isd/clifftop_retreat.rs` lines 19-23)
  - Oracle text says: `This land enters tapped unless you control a Mountain or a Plains.`
  - Code does: `o.subtypes.iter().any(|s| s == "Mountain") || o.subtypes.iter().any(|s| s == "Plains")` — but `o.subtypes` is always `Vec::new()` for non-token cards because `setup_game` (`engine.rs` lines 2677-2682) copies `keywords`, `card_types`, and `colors` to objects but never copies `subtypes`, and `create_object` (`state.rs` line 270) initializes `subtypes: Vec::new()`. For real Mountain/Plains cards the subtype lives only in the registry (`card_data.subtypes`). The correct pattern (as used in `check_condition` in `state.rs` lines 1085-1092) also checks `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == subtype)).unwrap_or(false)`, but this function never does that. Additionally, `controller_has_matching_land` does not accept `registry` as a parameter at all, so it cannot perform the registry check even in principle.

### Tricky interactions checked
- **"unless you control" evaluated at ETB**: PASS — implemented in `on_enter_battlefield`, which fires at the correct time via the `PendingTrigger::EnteredBattlefield` path in `triggers.rs` line 897
- **Self-exclusion during the check**: PASS — `o.id != object_id` on line 20 correctly excludes Clifftop Retreat itself from the scan (prevents counting itself as a Plains or Mountain)
- **"you control" scoping**: PASS — controller is obtained from the entering land object itself, and `objects_in_zone(Zone::Battlefield, controller)` correctly filters to that player's permanents only
- **Registry subtypes vs object subtypes for non-token lands**: FAIL — see code issue above; `o.subtypes` is empty for all non-token cards; registry subtypes are never consulted
- **Token lands with Mountain/Plains subtype**: PASS — tokens store subtypes on the object (`create_token_internal` sets `subtypes` explicitly), so the `o.subtypes` check does work for those hypothetical tokens
- **Mana ability produces {R} or {W}**: PASS — two separate `ManaAbilityDef` entries (ability_index 0 and 1) for Red and White; player can choose either
- **Mana ability only available when untapped and on battlefield**: PASS — guarded by `obj.zone == Zone::Battlefield && !obj.tapped` on line 57
- **Test masking the bug**: FAIL — the test `clifftop_retreat_enters_untapped_with_mountain` (line 51) manually sets `state.get_object_mut(mtn).unwrap().subtypes = vec!["Mountain".into()]` (line 59), which is not what happens in real gameplay; this causes the test to pass while the underlying production code path is broken

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card enters tapped without matching lands: `mtg-engine/tests/innistrad_simple_cards.rs:31`
- Card enters untapped with Mountain present (test rigs object subtypes manually, masking bug): `mtg-engine/tests/innistrad_simple_cards.rs:51`
- Mana abilities produce {R} or {W}: `mtg-engine/tests/innistrad_simple_cards.rs:76`
- Registry subtype check (Mountain/Plains via `card_data`, not `o.subtypes`): NOT TESTED
- Full end-to-end game flow where Mountain is played normally (without manually setting `o.subtypes`): NOT TESTED
- Nonbasic lands with basic land types (e.g., Sacred Foundry): NOT TESTED
