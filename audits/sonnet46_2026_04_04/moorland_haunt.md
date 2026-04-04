## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.
**Type line**: Land
**Status**: ISSUE

### Code issues

- Player cannot choose which creature card to exile when multiple are in the graveyard (`mtg-engine/src/cards/isd/moorland_haunt.rs` lines 85–96 and `mtg-engine/src/engine.rs` lines 399–406)
  - Oracle text says: `Exile a creature card from your graveyard:`
  - Code does: In `on_activate_ability`, auto-selects the first creature card found via `.next()` on a `HashMap` iterator (non-deterministic order). In `legal_actions`, a single `ActivateAbility { targets: vec![] }` action is generated regardless of how many creature cards are in the graveyard, providing no mechanism for the player to indicate which card to exile. When there are multiple creature cards in the graveyard, the player is denied the choice that the oracle text requires.

### Tricky interactions checked

- Player choice for which creature to exile when multiple are in graveyard: FAIL — `on_activate_ability` uses `.next()` to auto-select the first creature card found (lines 85–89); `legal_actions` always generates a single untargeted action (no per-creature action) so the choice cannot be expressed.
- Token has correct characteristics (1/1, white, Flying, Spirit subtype, creature type): PASS — `create_token_with_subtypes("Spirit Token", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()])` matches oracle exactly.
- Mana ability ({T}: Add {C}) gated on battlefield presence and untapped state: PASS — `mana_abilities` checks `obj.zone == Zone::Battlefield && !obj.tapped`.
- Activated ability gated on battlefield presence, untapped state, and at least one creature card in graveyard: PASS — `activated_abilities` returns `vec![]` when tapped or off-battlefield; returns `vec![]` when no creature card is in graveyard.
- Mana cost of activated ability is {W}{U}: PASS — `ManaCost::new(vec![ManaSymbol::Colored(Color::White), ManaSymbol::Colored(Color::Blue)])`.
- Tap cost is required for activated ability: PASS — `requires_tap: true`.
- Activated ability is instant-speed (not sorcery-speed only): PASS — `sorcery_speed_only: false`.
- Activated ability can be used multiple times per turn: PASS — `once_per_turn: false`.
- Exile goes to Exile zone (not Graveyard): PASS — `state.move_object(exile_id, Zone::Exile)`.
- Token does not enter from an existing card (`is_token: true`): PASS — `create_token_with_subtypes` sets `is_token: true` in `create_token_internal`.
- Parallel Lives doubling interacts correctly with token creation: PASS — `create_token_with_subtypes` checks for Parallel Lives and doubles accordingly.
- Creature card detection correctly excludes tokens from the graveyard: PASS — filter uses `!o.is_token`.

### Test coverage

- Basic card data (Land type, oracle text contains "Spirit"): `innistrad_simple_cards.rs:205` (moorland_haunt_card_data)
- Ability activation creates 1/1 white Spirit token with Flying and exiles the creature: `innistrad_simple_cards.rs:214` (moorland_haunt_creates_spirit_token) — but only tested with a single creature in the graveyard; the no-choice case trivially passes
- Player choice of which creature to exile when multiple are in graveyard: NOT TESTED
- Ability unavailable when Moorland Haunt is tapped: NOT TESTED
- Ability unavailable when no creature card is in graveyard: NOT TESTED
- Multiple activations in the same turn: NOT TESTED
