## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
**Type line**: Artifact
**Status**: ISSUE

### Code issues

- **No player choice when multiple basic lands exist** (`mtg-engine/src/cards/isd/travelers_amulet.rs:57`)
  - Oracle text says: `Search your library for a basic land card`
  - Code does: `player.library_order.iter().find(|&&lib_id| { ... })` — auto-selects the first matching basic land in library order, never presenting a choice to the player. The engine has a `ChooseFromLibrary` resolution choice mechanism (used correctly by Garruk, the Veil-Cursed in `engine.rs`) that allows the player to pick from eligible cards. Traveler's Amulet bypasses this entirely. If a player's library contains both a Forest and a Mountain (or any combination of basic lands), the card always takes the first one found, denying the player a meaningful search choice.

- **"then shuffle" is not implemented** (`mtg-engine/src/cards/isd/travelers_amulet.rs:83`)
  - Oracle text says: `then shuffle`
  - Code does: `// Shuffle (no-op in our engine, library is treated as ordered for gameplay).` — no shuffle is performed. This comment is factually incorrect: the engine supports real shuffling via `rand::seq::SliceRandom`. The `ChooseFromLibrary` handler in `engine.rs` (lines 2044–2049) shuffles the library after every search: `new_state.get_player_mut(*searcher).library_order.shuffle(&mut rng)`. Garruk, the Veil-Cursed (lines 2393–2407) also shuffles, even when no card is found. Traveler's Amulet does neither.

### Tricky interactions checked

- **Player choice during search**: FAIL — auto-selects first match, should present `ChooseFromLibrary` choice when multiple basic lands are present.
- **Shuffle after search**: FAIL — no shuffle performed despite the oracle requiring it and the engine supporting it (see `engine.rs` lines 2044–2049, 2393–2407).
- **Shuffle when no basic land found**: FAIL — no shuffle performed even in the "nothing found" branch (line 80–82). Garruk's analogous code explicitly shuffles even on failure (line 2393–2395); Traveler's Amulet does not.
- **Controller retrieval after sacrifice**: PASS — `state.get_object(object_id)` still returns the sacrificed object (zone changed to Graveyard but object is retained in `state.objects`); controller field is not cleared by `move_object`.
- **Sacrifice cost payment order**: PASS — engine pays sacrifice cost (`destruction::sacrifice`) before calling `on_activate_ability`, consistent with MTG rules (costs are paid before effects resolve).
- **Sorcery-speed timing**: PASS — `sorcery_speed_only: false` is correct; oracle text has no timing restriction on the ability.
- **Basic land identification**: PASS — checks `d.card_types.contains(&CardType::Land) && d.supertypes.contains(&Supertype::Basic)` via registry. Tokens can't be in libraries, so registry-only check is fine here.
- **Mana cost**: PASS — `ManaCost::new(vec![ManaSymbol::Generic(1)])` matches `{1}`.
- **Card type**: PASS — `card_types: vec![CardType::Artifact]` matches "Artifact" type line.
- **"reveal it" step**: The log message `"Traveler's Amulet: p{} searched for {}"` announces the card name publicly, which serves as the digital equivalent of reveal. This is not flagged as an issue given how the engine handles visibility.
- **`once_per_turn` flag**: PASS — `once_per_turn: false` is correct; the ability has no "activate only once per turn" clause.

### Test coverage

- Basic card data (type, cost): `tier9_cards.rs:23` — TESTED
- Ability finds a basic land and moves it to hand: `tier9_cards.rs:33` — TESTED
- Player choice between multiple basic land types: NOT TESTED
- Shuffle after successful search: NOT TESTED
- Shuffle after failed search (no basic land in library): NOT TESTED
- Ability activation at non-main-phase (instant speed): NOT TESTED
