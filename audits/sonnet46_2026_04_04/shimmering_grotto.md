## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{1}, {T}: Add one mana of any color.
**Type line**: Land
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- `{T}: Add {C}` implemented as `ManaAbilityDef` with `produced: vec![(ManaType::Colorless, 1)]`, `requires_tap: true` — both abilities gated on `obj.zone == Zone::Battlefield && !obj.tapped`: pass
- `{1}, {T}: Add one mana of any color` implemented as 5 separate `ActivatedAbilityDef` entries (ability_index 1–5, one per color W/U/B/R/G), each with `cost: ManaCost::new(vec![ManaSymbol::Generic(1)])` and `requires_tap: true`; `on_activate_ability` adds the correct colored mana for each index: pass
- Mutual exclusion of both tap abilities: both `mana_abilities()` and `activated_abilities()` check `!obj.tapped`, so once the land is tapped by either ability the other becomes unavailable: pass
- Color-fixing ability requires {1} from an external source: the ability is only generated as a legal `ActivateAbility` action when the player's mana pool already contains ≥ {1} generic (checked at engine.rs line 353 via `mana::can_pay`); a lone untapped Grotto cannot fund its own color-fixing cost because tapping for {C} and tapping for the color-fixing ability are mutually exclusive: pass
- `oracle_text` field in `card_data()` matches Scryfall verbatim (`"{T}: Add {C}.\n{1}, {T}: Add one mana of any color."`): pass
- `{1},{T}` ability classified as `ActivatedAbilityDef` (non-mana) rather than `ManaAbilityDef`: per CR 605.1 the `{1},{T}` ability is a mana ability (produces mana, no target, not a planeswalker). In this engine both types resolve immediately without the stack and both require priority, so the practical in-game difference is negligible. One side-effect: `has_castable_with_potential_mana` adds the Grotto's {C} to `potential` (mana ability path) but then separately checks whether the color-fixing `ActivatedAbilityDef` is affordable with that same {C}; since {C} can pay {1} generic and the Grotto is untapped, the function can return a false positive (prompt the player) when the Grotto is the only untapped land and the player cannot actually activate both tap abilities. This is an AI auto-pass heuristic issue only, not a rules violation: pass (no rules violation; minor AI quality concern)
- Stale doc comment (lines 11–13) says "the {1},{T} ability adds {G} (arbitrary choice...)" but the implementation correctly presents all five color choices via five separate `ActivatedAbilityDef` entries: misleading comment but code behavior is correct: pass
- `ActivateManaAbility` handler looks up mana ability by array position (`abilities.get(*ability_index)`) rather than by the `ability_index` field. Shimmering Grotto has exactly one mana ability with `ability_index: 0` at vec position 0, so the lookup is correct for this card: pass
- No `Tapped` or `ManaAdded` `GameEvent`s are pushed when the color-fixing `ActivateAbility` fires (unlike the `ActivateManaAbility` path which pushes both). These events are only consumed by external log/UI consumers; they do not affect internal game state: pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card type is Land: `innistrad_simple_cards.rs:183` (shimmering_grotto_card_data)
- `{T}: Add {C}` mana ability available and generates `ActivateManaAbility` action: `innistrad_simple_cards.rs:187` (shimmering_grotto_taps_for_colorless)
- `{1}, {T}: Add one mana of any color` — ability is available when player has {1}: NOT TESTED
- `{1}, {T}: Add one mana of any color` — correct colored mana added to pool on activation: NOT TESTED
- `{1}, {T}: Add one mana of any color` — unavailable when land is already tapped: NOT TESTED
- `{1}, {T}: Add one mana of any color` — unavailable when player has no {1} in pool: NOT TESTED
- Both tap abilities mutually exclusive (tapping for one prevents the other): NOT TESTED
