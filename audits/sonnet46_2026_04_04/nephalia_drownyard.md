## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{1}{U}{B}, {T}: Target player mills three cards.
**Type line**: Land
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- `{T}: Add {C}` colorless mana ability — `mana_abilities` returns a `ManaAbilityDef` with `produced: vec![(ManaType::Colorless, 1)]` and `requires_tap: true`; engine taps the land on activation (engine.rs:1677). pass
- `{1}{U}{B}, {T}` cost structure — `activated_abilities` returns `ActivatedAbilityDef` with `ManaSymbol::Generic(1)`, `ManaSymbol::Colored(Color::Blue)`, `ManaSymbol::Colored(Color::Black)` and `requires_tap: true`; engine pays mana and sets `tapped = true` at engine.rs:1733–1740. pass
- Target player includes self — `TargetRequirement::PlayerOnly` in `generate_ability_targets` iterates `state.players` without filtering out the controller; `can_target_player` only blocks targeting if the target player has hexproof AND is not the caster, so the controller can target themselves. pass
- Target player includes opponent — same loop; both players appear as valid targets, each producing a separate `Action::ActivateAbility` entry in legal_actions. pass
- Mill count — `mill_cards(state, *player_id, 3)` loops exactly 3 times, moving the top card to `Zone::Graveyard` each iteration. pass
- Mill with fewer than 3 cards in library — `mill_cards` breaks early when `library_order` is empty; mills whatever remains without error. pass
- Ability only available when untapped — `activated_abilities` guards `obj.zone == Zone::Battlefield && !obj.tapped`; legal_actions also guards `ab.requires_tap && obj_tapped` (engine.rs:356). double-guarded, pass
- Mana ability only available when untapped — `mana_abilities` guards `obj.zone == Zone::Battlefield && !obj.tapped`. pass
- `once_per_turn: false` — ability has no once-per-turn restriction, matching oracle text which imposes no such limit. pass
- `keywords: vec![]` — Mill is a keyword action, not a keyword ability; absence from keywords vec is correct. pass
- `card_types: vec![CardType::Land]`, `cost: None` — matches oracle type line and the convention that lands have no mana cost. pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card data (Land type, no cost): `tier10_cards.rs:271` (`nephalia_drownyard_card_data`)
- Mills exactly 3 cards targeting opponent: `tier10_cards.rs:281` (`nephalia_drownyard_mills_three`)
- Legal action generated when mana is available: `tier10_cards.rs:304–309`
- Self-targeting (controller mills themselves): NOT TESTED
- Mill with fewer than 3 cards in library: NOT TESTED
- Ability unavailable when tapped: NOT TESTED
- Mana ability produces colorless: NOT TESTED
