## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

- **Colors never copied when creating token copy** — `mtg-engine/src/state.rs`, `create_token_copy` function (line 426)
  - Oracle text says: `"Create a token that's a copy of target creature you control."` — a copy must have all the copiable values of the source, including color.
  - Code does: `Vec::new(), // colors TODO` — colors are hardcoded to empty regardless of the source creature's colors. In a real game, source creatures have colors derived from their mana cost (set in `setup_game` at line 2678). A token copy of a black creature would have no color, making it targetable by Doom Blade (`!o.colors.contains(&Color::Black)` in `doom_blade.rs` line 40) when it should be immune.

- **Copying a token source loses its card_types, keywords, and subtypes** — `mtg-engine/src/state.rs`, `create_token_copy` function (lines 424–431)
  - Oracle text says: `"Create a token that's a copy of target creature you control."` + Ruling: `"If the copied creature is a token, the token that's created copies the original characteristics of that token as stated by the effect that created the token."`
  - Code does: `let (colors, keywords, card_types, subtypes) = registry.card_data(card_id).map(...).unwrap_or_default();` — for token sources, `card_id = CardId(0)` (the sentinel set in `create_token_internal` line 365). `registry.card_data(CardId(0))` returns `None` (the registry starts at `next_id: 1`, so `CardId(0)` is never registered). `unwrap_or_default()` yields `([], [], [], [])`. A copy of a Flying Spirit token (e.g., from Midnight Haunting) gets `keywords = []`, `card_types = []`, `subtypes = []`. The copy lacks Flying, has no Creature type, and has no Spirit subtype — all wrong. The `has_keyword` check first tests `obj.keywords.contains(&keyword)` (empty) then falls back to `registry.get(obj.card_id)` (CardId(0), not registered, returns None), so Flying is never found.

### Tricky interactions checked

- **Flashback exile**: `move_spell_after_resolve` checks `cast_with_flashback` flag and sends to Exile or Graveyard accordingly — PASS. `cast_with_flashback` is set at cast time in `engine.rs` line 1637.
- **Target legality at resolution**: Code checks `o.zone == Zone::Battlefield` at resolve time before creating the token. If the target left the battlefield, the token is not created (effect does nothing) but `move_spell_after_resolve` still fires unconditionally — PASS. Correct MTG behavior.
- **Controller of token**: Token owner is taken from the Cackling Counterpart spell's controller (`state.get_object(object_id).map(|o| o.controller)`) — PASS.
- **ETB triggers on the token copy**: `create_token_internal` (called via `create_token_with_subtypes`) emits `EnteredBattlefield` at lines 404–407 and calls `apply_entering_copy_replacement`. Additionally, `create_token_copy` sets `obj.card_id = source_card_id` after creation (lines 444–446), so the token can look up the source's `CardBehavior` (including its ETB behavior) when the event is processed — PASS for non-token sources.
- **ETB triggers for copy of a token**: The token copy of a token gets `card_id = CardId(0)` (since source also has `card_id = CardId(0)`). `registry.get(CardId(0))` returns `None`, so no behaviors fire. If the source token had ETB triggers expressed via a named card behavior, they would not fire — but typical game tokens (Spirit, Wolf, Zombie) have no ETB abilities, so this is a minor practical issue, though technically wrong per the ruling.
- **Copying a non-token creature's keywords (e.g., Flying)**: For regular non-token sources, `registry.card_data(card_id)` returns correct keywords. The copy token gets keywords stored on `obj.keywords`. `has_keyword` checks `obj.keywords` first, finds them — PASS for non-token sources.
- **"You may" / optionality**: Cackling Counterpart's effect is not optional ("Create a token…" not "You may create a token"). The code unconditionally calls `create_token_copy` when the target is valid — PASS.
- **Parallel Lives doubling**: `create_token_with_subtypes` checks for Parallel Lives and creates extra copies as appropriate — PASS. This interacts correctly with Cackling Counterpart.
- **Flashback exiles if countered**: The ruling states "A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way." The `flashback_spell_countered_is_exiled` test in `flashback.rs:127` verifies this via `move_spell_after_resolve` — PASS.
- **Colors affecting Doom Blade targeting**: A copy of a black creature should not be targetable by Doom Blade. Due to the colors-not-copied bug, it would be incorrectly targetable — FAIL (tied to Issue 1 above).
- **Colors affecting Intimidate blocking**: Intimidate check at `combat.rs:640` uses `attacker.colors` and `blocker.colors`. A token copy created by Cackling Counterpart always has `colors = []`, making Intimidate calculations wrong if the source creature was colored — FAIL (tied to Issue 1 above).

### Test coverage

- Basic token copy (name, p/t, is_token): `tier12_cards.rs:487` — TESTED
- Token has correct keywords (e.g., Flying from Chapel Geist): NOT TESTED
- Token has correct colors: NOT TESTED
- Token has correct card_types: NOT TESTED
- Token has correct subtypes: NOT TESTED
- Copying a token source (token-of-token copy): NOT TESTED
- Copy of black creature cannot be targeted by Doom Blade: NOT TESTED
- Flashback cost is `{5}{U}{U}`: `tier12_cards.rs:510` — TESTED
- Flashback spell exiled after resolution: covered by system test in `flashback.rs:86` (uses Geistflame, not Cackling Counterpart specifically) — TESTED (generic)
- Flashback spell exiled if countered: `flashback.rs:129` — TESTED (generic)
- ETB trigger fires on token copy: NOT TESTED
