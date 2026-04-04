## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever another creature dies, you may draw a card. If you do, discard a card.
**Type line**: Creature — Bird
**Status**: ISSUE

### Code issues

- Simultaneous death: Murder of Crows' triggered ability does not fire when it dies at the same time as another creature — `mtg-engine/src/triggers.rs:418-440`
  - Oracle text says: `"Whenever another creature dies, you may draw a card. If you do, discard a card."` and the ruling says: `"If another creature dies at the same time as Murder of Crows, its last ability triggers."`
  - Code does: In `collect_triggers`, the death-watch watcher scan is `filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)`. In `destruction.rs:destroy()`, the `CreatureDied` event is pushed and then `state.move_object(id, Zone::Graveyard)` is called immediately for each dying creature in sequence. By the time `collect_triggers` processes all queued `CreatureDied` events (after all simultaneous deaths have resolved), Murder of Crows has already been moved to `Zone::Graveyard` and thus fails the `Zone::Battlefield` filter. The trigger is therefore never collected for the other creature's simultaneous death.

### Tricky interactions checked

- "another" filter (self-exclusion): PASS — `collect_triggers` filters `o.id != dead_id` so Murder of Crows does not watch its own death event as a DeathWatch watcher.
- "another" + simultaneous death (same board wipe): FAIL — See issue above. All zone changes happen before `collect_triggers` runs, so Murder of Crows is already in the graveyard when the scan for watchers occurs, and its trigger for the simultaneously-dying creature is never collected.
- "you may" optionality: PASS — `on_any_creature_dies` sets `AwaitingAction::ResolutionChoice` with `ResolutionChoiceKind::YesNo`, correctly presenting the choice to the player rather than auto-selecting.
- "If you do, discard a card" (draw-then-discard atomicity): PASS — `on_yes_no_choice` draws first, then either auto-discards the single card (if exactly 1 in hand after draw) or presents a `ChooseCardFromHand` choice before any other priority window opens.
- Player cannot act between draw and discard: PASS — Both the draw and the discard choice are resolved within the same `on_yes_no_choice` callback; no priority is passed back to players in between.
- "you may" with empty library: PASS — `draw_cards` with empty library draws 0 cards; `hand.len() == 0` falls through both branches, so no discard choice is presented (correctly satisfying "if you do").
- Murder of Crows leaving battlefield before trigger resolves: PASS — `resolve_next_trigger` for `DeathWatch` guards with `o.zone == Zone::Battlefield` before calling `on_any_creature_dies`, and `on_any_creature_dies` itself also guards with the same check, returning early if Murder of Crows is not on the battlefield.
- controller identity in `on_yes_no_choice`: PASS — `state.get_object(self_id)` returns the object from `state.objects` regardless of zone (objects are not removed on zone change in this engine), so the correct controller is retrieved even after Murder of Crows has left the battlefield.
- Flying keyword declared: PASS — `keywords: vec![Keyword::Flying]` matches oracle text.
- Mana cost and P/T: PASS — `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Blue), ManaSymbol::Colored(Color::Blue)])` and `power: Some(4), toughness: Some(4)` match oracle {3}{U}{U} 4/4.
- Subtype: PASS — `subtypes: vec!["Bird".into()]` matches type line Creature — Bird.
- Oracle text field: PASS — `oracle_text` field verbatim matches the oracle text.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Trigger fires when another (non-simultaneous) creature dies: `mtg-engine/tests/card_fixes.rs:209` (`murder_of_crows_presents_draw_choice`)
- "you may" yes/no choice presented (not auto-resolved): `mtg-engine/tests/card_fixes.rs:209` (`murder_of_crows_presents_draw_choice`)
- If yes chosen, player draws and then must discard: NOT TESTED (the test only checks that the awaiting_action is set, not the full draw+discard resolution)
- If no chosen, no draw and no discard: NOT TESTED
- If another creature dies at the same time as Murder of Crows, ability triggers: NOT TESTED
- You can't do anything between drawing and discarding: NOT TESTED
- Trigger does not fire when Murder of Crows itself is the dying creature: NOT TESTED
- Trigger suppressed if Murder of Crows leaves battlefield before resolution: NOT TESTED
