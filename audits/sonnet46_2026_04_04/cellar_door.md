## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
**Type line**: Artifact
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Bottom vs top of library**: `library_order[0]` is the top (confirmed: `draw_top_card` uses `library_order.remove(0)`, and Delver of Secrets checks `library_order.first()` for the top card). Cellar Door mills `library_order[last_idx]` (last element), which is correctly the bottom card — pass.
- **"Target player" includes self**: `TargetRequirement::PlayerOnly` in `generate_ability_targets` generates `Target::Player(p.id)` for all non-lost players including the controller — pass.
- **"you create" — correct controller receives token**: `on_activate_ability` captures `controller` from the Cellar Door object's controller field and passes it to `create_token_with_subtypes`. This correctly gives the token to the Cellar Door's controller, not the targeted player — pass.
- **Creature card check vs tokens in library**: The check uses `registry.card_data(o.card_id)` to detect `CardType::Creature`. Tokens have `card_id: CardId(0)` and their types are stored on the object, not the registry — but tokens cease to exist when they would change zone to a library, so a library card will never be a token. The registry check is sufficient — pass.
- **Empty library**: Code guards with `if player.library_order.is_empty() { return; }` before attempting to remove — pass.
- **Behavior dispatch after tapping**: After paying the tap cost (`obj.tapped = true`), the engine re-calls `activated_abilities` to find `behavior_card_id`. Since the object is now tapped, Cellar Door's `activated_abilities` returns `vec![]`. The engine falls through the attached-aura check, finds no aura, and falls back to `unwrap_or(card_id)`, so `on_activate_ability` is called on the correct behavior — pass.
- **Timing restriction**: `sorcery_speed_only: false` and `once_per_turn: false` — correct, the ability has no inherent timing restriction beyond priority — pass.
- **Ability available only when untapped**: `activated_abilities` returns the ability only when `!obj.tapped`, and the engine additionally guards with `if ab.requires_tap && obj_tapped { continue; }`. Both checks agree — pass.
- **Token stats and types**: Token created with `name="Zombie"`, `power=2`, `toughness=2`, `colors=[Black]`, `card_types=[Creature]`, `subtypes=["Zombie"]` — matches oracle "2/2 black Zombie creature token" — pass.
- **Parallel Lives interaction**: `create_token_with_subtypes` correctly doubles tokens when Parallel Lives is on the battlefield under the controller — pass (handled generically by the state layer, not the card).

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Mills bottom creature card → creates Zombie token: `mtg-engine/tests/tier15_cards.rs:607` (`cellar_door_creates_zombie_when_milling_creature`) — TESTED (single-card library, so top = bottom; covers the creature-detected path)
- Mills non-creature card → no token created: NOT TESTED
- Empty library → does nothing: NOT TESTED
- Target player is opponent vs self: NOT TESTED
- Ability unavailable when already tapped: NOT TESTED
- Parallel Lives doubling of the token: NOT TESTED
