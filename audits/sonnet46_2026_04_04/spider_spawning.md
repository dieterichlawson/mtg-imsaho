## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Flashback exile (not graveyard) after resolution**: `move_spell_after_resolve` checks `cast_with_flashback` flag and calls `move_object(object_id, Zone::Exile)` when true. The flag is set at cast time in `engine.rs:1636-1637` (`obj.cast_with_flashback = true`). Confirmed correct.

- **Spell zone at resolution time**: When cast via flashback, the engine moves the spell from `Zone::Graveyard` to `Zone::Stack` at line 1632 (`new_state.move_object(*object_id, Zone::Stack)`) before any resolution happens. So when `on_resolve` runs, Spider Spawning is in `Zone::Stack`, not `Zone::Graveyard`. The graveyard filter `o.zone == Zone::Graveyard` already excludes it. The `o.id != object_id` guard is redundant but harmless.

- **Count at resolution, not at cast**: The count is computed inside `on_resolve`, which is called during resolution after any priority action between cast and resolution. This matches the ruling "The number of creature cards in your graveyard is counted when Spider Spawning resolves."

- **"creature card" detection via `power.is_some()`**: The code uses `o.power.is_some()` as a proxy for "creature card" rather than checking `o.card_types.contains(&CardType::Creature)` or `registry.card_data(o.card_id).map(|d| d.card_types.contains(&CardType::Creature))`. Verified that `setup_game` (engine.rs:2670-2682) initializes all objects with `power: card_data.power`, so every creature card starts with `power: Some(n)` and every non-creature card starts with `power: None`. The `move_object` function does not clear `power` on zone changes. Therefore `power.is_some()` correctly identifies creature cards for all currently-implemented cards.

- **Token not counted as creature card**: Tokens that die leave the game via SBA rule 704.5d (sba.rs:308-315: tokens not on the battlefield are removed from `state.objects`). They will never appear in the graveyard to be counted.

- **"your graveyard" — owner filter**: The code filters by `o.owner == controller` where `controller = state.get_object(object_id).map(|o| o.controller)`. In MTG, "your graveyard" means the controller's graveyard (cards owned by the controller). The `owner` field is set at object creation and never changes, matching correct MTG graveyard semantics.

- **Token attributes (1/2, green, Reach, Spider subtype)**: Token is created as `create_token_with_subtypes("Spider", controller, 1, 2, vec![Color::Green], vec![CardType::Creature], vec![Keyword::Reach], vec!["Spider".into()])`. All attributes match oracle text.

- **Flashback timing (sorcery speed only)**: The `legal_actions` generation (engine.rs:692-706) checks `is_sorcery_type` for sorceries and gates casting on `is_sorcery_speed`. Correct.

- **Flashback countered → still exiled**: Tested in `flashback.rs:129` (`flashback_spell_countered_is_exiled`). The `cast_with_flashback` flag is set on the object when cast; the stack-resolution path for countering calls through the same `move_spell_after_resolve` pathway. Pass (verified for other flashback cards; the mechanic is generic).

- **Self-reference exclusion**: Spider Spawning is a Sorcery with `power: None`. Even without the `o.id != object_id` guard, Spider Spawning itself would never pass `o.power.is_some()`. The guard is present and redundant, but introduces no incorrect behavior.

- **Parallel Lives interaction**: `create_token_with_subtypes` checks for Parallel Lives on the battlefield (state.rs:326-345) and doubles token creation. Not specifically a Spider Spawning concern, but the mechanism applies correctly here.

### Test coverage

- **Basic functionality (N creature cards → N Spider tokens with 1/2 P/T)**: `tier5_cards.rs:187` (`spider_spawning_creates_tokens`) — TESTED
- **Spider tokens have Reach keyword**: NOT TESTED (code sets `Keyword::Reach` in keywords vec, but test only checks P/T)
- **Spider tokens are Green**: NOT TESTED
- **Spider tokens have Spider subtype**: NOT TESTED
- **Flashback cast exiles Spider Spawning**: NOT TESTED specifically for Spider Spawning (general flashback exile tested in `flashback.rs:86` for Geistflame and `flashback.rs:471` for Bump in the Night)
- **Count evaluated at resolution time, not cast time**: NOT TESTED
- **Flashback cost {6}{B} is offered and enforced**: NOT TESTED specifically for Spider Spawning
- **Creature in graveyard added after cast but before resolution is counted**: NOT TESTED
