## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Each player chooses a creature they control. Destroy the rest.
Flashback {5}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **APNAP order (ruling: "Starting with the player whose turn it is, each player chooses a creature in turn order")**: PASS. `on_resolve` rotates `player_order` to begin with `state.active_player`, then processes each player in sequence.
- **Players with 0 creatures not asked to choose**: PASS. The `creatures.len() <= 1` branch skips players with 0 creatures; they are not added to `pending_players` and nothing is added to `kept` for them.
- **Players with 1 creature auto-kept (no choice needed)**: PASS. The `creatures.len() <= 1` branch calls `creatures.first()` and pushes the single creature into `kept` with a log message, no choice prompt.
- **"Destroy the rest" uses `try_destroy` (respects indestructible and regeneration shields)**: PASS. Both the initial destruction path (line 81 of `divine_reckoning.rs`) and the engine's `KeepOneDestroyRest` handler (engine.rs lines 2491, 2523) call `crate::destruction::try_destroy`, which checks indestructible and regeneration before destroying.
- **Flashback exile after resolution**: PASS. `on_resolve` calls `state.move_spell_after_resolve(object_id)` (line 71), which checks `cast_with_flashback` and routes to `Zone::Exile` if true; `cast_with_flashback` is set to `true` in `engine.rs` line 1637 when the card is cast from the graveyard.
- **Flashback spell exiled even when countered (ruling: "A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way")**: PASS. `LostInTheMist.on_resolve` calls `state.move_spell_after_resolve(*spell_id)` on the countered spell (lost_in_the_mist.rs line 56), which correctly routes a flashback-cast spell to exile. The `flashback_spell_countered_is_exiled` test in `flashback.rs` covers this for Geistflame; the mechanism is shared.
- **`source: ObjectId(0)` used in chained KeepOneDestroyRest awaiting_action**: PASS. The `choice_source` field is only read in the `ChooseCardFromHand` branch (engine.rs line 2020) for `on_discard_choice`; the `ChooseTarget`/`KeepOneDestroyRest` path never references it. Using `ObjectId(0)` for chained choices is harmless.
- **Multi-player chains (3+ players)**: PASS. The loop in the `KeepOneDestroyRest` engine handler (lines 2513–2558) correctly iterates over remaining players, auto-keeping those with 0–1 creatures and presenting a choice prompt for those with 2+.
- **Players will know earlier choices**: PASS (via log). Earlier choices are logged as `"Divine Reckoning: p{} keeps {}"` (line 2481). Subsequent choosers can see the log. The `AwaitingAction` description does not embed the prior choices directly, but in-game information is available.
- **Flashback cost value ({5}{W}{W} = 7 MV)**: PASS. `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(5), ManaSymbol::Colored(Color::White), ManaSymbol::Colored(Color::White)]))` — confirmed by `divine_reckoning_has_flashback` test asserting MV == 7.
- **Flashback timing (sorcery-speed only)**: PASS. `engine.rs` lines 692–706 gate graveyard casts by sorcery timing for non-instant/non-flash cards; Divine Reckoning is a Sorcery.
- **Choice options correctly scoped to the choosing player's creatures**: PASS. Both `on_resolve` (line 89–92) and the engine's chain handler (lines 2499–2501, 2530–2532) filter `o.controller == <that_player>`.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic each-player-chooses-one behavior: `tier8_cards.rs:255` (`divine_reckoning_keeps_one_per_player`) — TESTED
- Auto-keep single creature / no choice for 0-creature player: `tier8_cards.rs:313` (`divine_reckoning_with_one_creature_keeps_it`) — TESTED
- Flashback cost present and correct MV: `tier8_cards.rs:336` (`divine_reckoning_has_flashback`) — TESTED
- Flashback spell exiled after resolution: NOT TESTED specifically for Divine Reckoning (covered generally by `flashback.rs:86` for Geistflame)
- Flashback countered → exiled: NOT TESTED for Divine Reckoning (covered generally by `flashback.rs:129` for Geistflame)
- Indestructible creature not destroyed by "Destroy the rest": NOT TESTED
- Regenerating creature survives "Destroy the rest": NOT TESTED
- APNAP order (non-active player second): tested implicitly by `divine_reckoning_keeps_one_per_player` (P0 active, P0 chooses first, P1 second) — TESTED
- 3+ player scenario: NOT TESTED
