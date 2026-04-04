## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
**Type line**: Land
**Status**: ISSUE

### Code issues

- **"May" search is forced — controller gets no choice** (`mtg-engine/src/cards/isd/ghost_quarter.rs`, line 81–100)
  - Oracle text says: `"Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle."`
  - Code does: `// Its controller may search for a basic land (auto-search).` — immediately proceeds with `.find()` and puts the land onto the battlefield with no `AwaitingAction::ResolutionChoice` or `YesNo` prompt. The controller cannot decline.

- **Missing shuffle after library search** (`mtg-engine/src/cards/isd/ghost_quarter.rs`, lines 92–101)
  - Oracle text says: `"put it onto the battlefield, then shuffle."`
  - Code does: calls `state.move_object(land_id, Zone::Battlefield)` and returns; there is no call to `library_order.shuffle()`. Comparable search cards in the same engine (Caravan Vigil `caravan_vigil.rs:86`, Garruk Relentless `garruk_relentless.rs:56`, Bitterheart Witch `bitterheart_witch.rs:99`, Memory's Journey `memorys_journey.rs:76`) all call `library_order.shuffle()` after searching.

### Tricky interactions checked

- **"May" optionality** (controller can decline search): FAIL — code auto-searches unconditionally; no player choice is presented.
- **Shuffle after search**: FAIL — `library_order.shuffle()` is never called. Multiple comparable cards in the engine do shuffle; the omission is inconsistent and violates the oracle text.
- **Target legality at resolution** (ruling 2011-09-22: if target is illegal when ability resolves, nothing happens): PASS — code checks `o.zone == Zone::Battlefield` before proceeding (`ghost_quarter.rs:72-73`); returns early if target moved.
- **Self-targeting** (ruling 2006-05-01: targeting Ghost Quarter with its own ability doesn't resolve): PASS — sacrifice cost is paid by the engine before `on_activate_ability` is invoked (`engine.rs:1747-1748`), so Ghost Quarter is already in the graveyard when the zone check runs; the battlefield check at line 72-73 catches this and returns.
- **Indestructible / regenerated target still grants search** (ruling 2013-07-01): PASS — `try_destroy` return value is ignored (`ghost_quarter.rs:77`); the code unconditionally proceeds to the search regardless of whether destruction succeeded.
- **Target controller's library searched** (not the Ghost Quarter controller's): PASS — code captures `target_controller` from the target object (`ghost_quarter.rs:71`) and searches `state.get_player(target_controller).library_order` (`ghost_quarter.rs:82`).
- **Taps for {C} mana ability**: PASS — `mana_abilities` returns a `ManaAbilityDef` producing `(ManaType::Colorless, 1)` when untapped on battlefield (`ghost_quarter.rs:35-43`).
- **Sacrifice is the cost, not part of resolution**: PASS — `SacrificeCost::SacrificeThis` (`ghost_quarter.rs:57`) causes the engine to sacrifice Ghost Quarter before calling `on_activate_ability`.
- **Ability speed (instant vs sorcery)**: PASS — `sorcery_speed_only: false` is correct; Ghost Quarter's oracle text does not restrict to sorcery speed.
- **Target type filter (lands only)**: PASS — `TargetRequirement::PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Land]))` (`ghost_quarter.rs:58-60`) and `matches_target_filter` checks `obj.card_types.contains(t)` (`engine.rs:1392-1394`), which is populated from registry at object creation.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Basic card data (Land type, oracle_text contains "Destroy target land"): `innistrad_simple_cards.rs:152`
- Taps for {C}: `innistrad_simple_cards.rs:161`
- "May" optionality (controller can decline search): NOT TESTED
- Shuffle after search: NOT TESTED
- Target legality at resolution (no effect if target gone): NOT TESTED
- Self-targeting does not grant search: NOT TESTED
- Indestructible target still grants search: NOT TESTED
- Regenerated target still grants search: NOT TESTED
- Full ability activation (destroy land + search + battlefield entry): NOT TESTED
