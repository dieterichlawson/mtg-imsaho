## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Type line**: Artifact
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Target player" includes the controller (self-targeting): PASS — `can_target_player` in `engine.rs` allows self-targeting; `is_valid_target` only checks graveyard contents, not whether target is opponent. Controller targeting their own graveyard is legal and works correctly.
- "You gain 2 life" means controller, not targeted player: PASS — `on_activate_ability` captures `controller` from the shovel object's controller field (line 65); `ExileFromGraveyardGainLife { controller }` carries this through to `apply_pending_effect` in `engine.rs:2337`, which applies life gain to `controller`.
- Targeted player chooses which card (Scryfall ruling): PASS — with multiple cards, `awaiting_action` is set with `player: *target_player` (line 109), so the targeted player makes the choice. With a single card, auto-exile is mechanically equivalent (only one option exists); mandatory per oracle text, no "may" involved.
- Cannot target player with empty graveyard: PASS — `is_valid_target` (line 56-61) returns false for any player with no cards in graveyard zone; engine's `generate_ability_targets` filters via this function.
- Ability unavailable when shovel is tapped: PASS — `activated_abilities` checks `obj.tapped` (line 34) and returns empty; engine also checks `requires_tap && obj_tapped` at line 356 of `engine.rs`.
- Summoning sickness does not block the ability: PASS — engine's legal action generation for non-mana activated abilities (engine.rs ~line 309) has no summoning sickness check; summoning sickness only affects creatures and Graveyard Shovel is an artifact with no creature type.
- "If it's a creature card" check covers registry data: PASS — both the single-card path (lines 81-86) and `apply_pending_effect` (engine.rs:2338-2344) use `registry.card_data(o.card_id).map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature))).unwrap_or(o.power.is_some())`. Tokens are a non-issue here because tokens cease to exist when they would go to the graveyard (state-based actions), so no tokens appear in the graveyard.
- Graveyard filtered by owner (not controller): PASS — lines 70 and 58 both filter by `o.owner == *target_player`, which is correct; cards always go to their owner's graveyard in MTG.
- optional: false for multiple-card choice: PASS — `optional: false` (line 114) means the engine does not generate a `None` choice (engine.rs:199-201), so the targeted player must exile a card. This matches the mandatory oracle wording "exiles a card."
- Life gain triggers GameEvent::LifeChanged: PASS — both the single-card path (lines 97-101) and `apply_pending_effect` (engine.rs:2353-2356) push `GameEvent::LifeChanged`.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Ability targets players (not cards directly): `graveyard_shovel.rs:22` (`targets_player_not_card`) TESTED
- Single card in graveyard auto-exiled; creature → gain 2 life: `graveyard_shovel.rs:51` (`auto_exiles_single_card`) TESTED
- Non-creature card exiled → no life gain: `graveyard_shovel.rs:73` (`no_life_gain_for_non_creature`) TESTED
- Multiple cards → resolution choice presented to targeted player: `graveyard_shovel.rs:94` (`multiple_cards_creates_resolution_choice`) TESTED
- Resolution choice exiles chosen card and grants life: `graveyard_shovel.rs:122` (`resolution_choice_exiles_and_gains_life`) TESTED
- Cannot target player with empty graveyard: `graveyard_shovel.rs:157` (`cannot_target_player_with_empty_graveyard`) TESTED
- Ruling [2011-09-22]: targeted player chooses which card (choice presented to P1, not P0): `graveyard_shovel.rs:111-116` TESTED
- Controller targeting their own graveyard: NOT TESTED
- Summoning sickness does not block artifact activation: NOT TESTED (not a concern for artifacts; behavior is engine-generic)
