## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Each player mills a card.
**Type line**: Artifact
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Each player" scope**: The `on_activate_ability` collects all player IDs via `state.players.iter().map(|p| p.id).collect()` and calls `mill_cards(state, pid, 1)` for each. Both players (including the activating player) are correctly milled: pass.
- **Tap cost in `activated_abilities`**: The ability is returned only when `obj.zone == Zone::Battlefield && !obj.tapped`. The engine then sets `tapped = true` at line 1740 before calling `on_activate_ability`. Tap cost is correctly paid: pass.
- **Summoning sickness on artifact**: The `legal_actions` tap-cost check (engine.rs line 356) only tests `obj_tapped`, not `summoning_sick`. Summoning sickness only restricts creatures (MTG rule 302.6), so a freshly-played Bell can tap immediately. The engine correctly does not apply summoning sickness to this artifact's tap ability: pass.
- **`mill_cards` mechanics**: The function removes the first element of `library_order` and calls `state.move_object(card_id, Zone::Graveyard)`, which is correct for "mills a card" (top card to graveyard): pass.
- **Empty library**: `mill_cards` breaks the loop if `library_order.is_empty()`, so milling a player with no library cards gracefully does nothing (no state-based loss from milling is handled separately as a state-based action, not here): pass.
- **Mana cost {1}**: Card data declares `ManaCost::new(vec![ManaSymbol::Generic(1)])` — mana value 1: pass.
- **Instant speed**: `sorcery_speed_only: false` — the ability can be activated any time the player has priority, consistent with a {T} activated ability having no sorcery-speed restriction: pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card type is Artifact and mana value is 1: `innistrad_simple_cards.rs:337` (`ghoulcallers_bell_card_data`)
- Both players mill exactly one card on activation: `innistrad_simple_cards.rs:346` (`ghoulcallers_bell_mills_both_players`)
- Ability unavailable when tapped: NOT TESTED
- Ability unavailable when not on battlefield: NOT TESTED
- Activation on turn Bell enters (no summoning sickness for artifacts): NOT TESTED
- Milling with empty library does not crash: NOT TESTED
