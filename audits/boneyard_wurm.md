## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
**Type line**: Creature — Wurm
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
**Type line**: Creature — Wurm
**Status**: PASS

### Code issues
No issues found.

All card data fields verified against oracle text:
- Name: "Boneyard Wurm" -- matches
- Mana cost: {1}{G} = Generic(1), Colored(Green) -- matches
- Card types: [Creature] -- matches
- Subtypes: ["Wurm"] -- matches
- Power/Toughness: Some(0)/Some(0) as placeholder for \*/\* -- correct pattern (dynamic_pt overrides)
- Oracle text string: matches exactly
- Keywords: none -- correct
- dynamic_pt counts creature cards (via power.is_some() proxy) in controller's graveyard -- correct

### Tricky interactions checked
- Non-creature cards excluded from count: PASS -- filtering by `o.power.is_some()` correctly excludes instants, sorceries, enchantments, etc. which have `power: None`
- Ability works in all zones (ruling 2018-12-07): PASS -- `effective_power`/`effective_toughness` have no zone restriction; `dynamic_pt` is called regardless of what zone the Wurm is in, so it counts itself if in the graveyard
- Only counts controller's graveyard, not opponent's: PASS -- `objects_in_zone(Zone::Graveyard, controller)` filters by owner matching the controller, so opponent's creature cards are excluded
- Graveyard filtered by owner not controller (MTG rule 400.3): PASS -- `objects_in_zone` correctly uses `obj.owner` for graveyard zone

### Test coverage
- Basic P/T equals creature count in graveyard: `tier7_cards.rs:19` (boneyard_wurm_pt_equals_creatures_in_graveyard)
- P/T is 0/0 with empty graveyard: `tier7_cards.rs:26`
- P/T updates as creatures enter graveyard: `tier7_cards.rs:30-36`
- Non-creature cards excluded from count: NOT TESTED
- Self-counting when Boneyard Wurm is in graveyard (ruling 2018-12-07): NOT TESTED
- Opponent's creatures not counted: NOT TESTED
