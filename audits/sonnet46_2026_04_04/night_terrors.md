## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target player reveals their hand. You choose a nonland card from it. Exile that card.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- **Night Terrors is never moved off the stack when the target player has multiple nonland cards in hand** (`mtg-engine/src/cards/isd/night_terrors.rs:63-70`, `mtg-engine/src/engine.rs:2003-2008`)
  - Oracle text says: `"Exile that card."` — Night Terrors must fully resolve (including cleanup to graveyard) after exiling the chosen card.
  - Code does: When `nonland_cards.len() > 1`, `on_resolve` calls `present_target_choice(... PendingEffect::ExileAndStore ... false)` and returns early (line 70) without calling `move_spell_after_resolve`. When the player later submits their choice, the engine's `ChooseTarget` resolution arm (engine.rs:2003-2008) calls `apply_pending_effect` but does NOT call `move_spell_after_resolve`. The `ExileAndStore` handler in `apply_pending_effect` (engine.rs:2255-2263) also does not call `move_spell_after_resolve`. Night Terrors remains on the stack indefinitely after the card is exiled.

- **Wrong `PendingEffect` variant used for Night Terrors** (`mtg-engine/src/cards/isd/night_terrors.rs:66`)
  - Oracle text says: `"Exile that card."` — Night Terrors is a sorcery with no LTB ability; it simply exiles the chosen card permanently.
  - Code does: Uses `PendingEffect::ExileAndStore { source_id: object_id, source_name: "Night Terrors".into() }`. The `ExileAndStore` handler (engine.rs:2259-2261) writes `source_obj.card_state.insert("exiled_creature".into(), *id)` onto the Night Terrors object — storage intended for permanents like Fiend Hunter that return the exiled card on LTB. Night Terrors has no LTB trigger and never reads this data. While the exile effect itself is correct, the spurious write misrepresents Night Terrors as an "exile-and-return" effect carrier.

### Tricky interactions checked

- **Targeting self (controller targets themselves)**: PASS. `can_target_player` (engine.rs:772-776) only blocks hexproof for non-self targets (`target_player != caster`), so a player can target themselves. `objects_in_zone(Zone::Hand, *target_player)` correctly reads the controller's own hand. The ruling "you must reveal your entire hand" is honored by enumerating all hand cards.
- **Empty hand / all lands in hand**: PASS. If `nonland_cards` is empty (whether hand is empty or contains only lands), the code logs the no-nonland message and falls through to `move_spell_after_resolve(object_id)` at line 73. Night Terrors resolves cleanly.
- **Single nonland card in hand (auto-selection)**: PASS. The `else if nonland_cards.len() == 1` branch directly exiles the card (line 57-61) without going through `present_target_choice`, then falls through to `move_spell_after_resolve`. Correct.
- **Multiple nonland cards in hand (choice presented)**: FAIL. As described above, Night Terrors is never cleaned up off the stack after the player's choice is applied. The card is stuck on the stack permanently.
- **"You choose" vs. "target player chooses"**: PASS. The oracle says "You choose" (the controller of Night Terrors chooses). The code passes `controller` (Night Terrors' controller) as the choosing player to `present_target_choice`, not the target player.
- **Mandatory choice (no "may")**: PASS. `present_target_choice` is called with `optional: false` (line 69), matching the mandatory "You choose" oracle text.
- **Exile vs. discard**: PASS. The card is moved to `Zone::Exile`, not `Zone::Graveyard`.
- **Land-type detection**: PASS. `registry.card_data(o.card_id).map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Land)))` correctly filters lands. For tokens with no registry entry, `card_data` returns `None` → `unwrap_or(false)` → treated as nonland, which is correct (tokens in hand are not lands).
- **Mana cost `{2}{B}`**: PASS. `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Black)])` matches oracle.
- **Sorcery type**: PASS. `card_types: vec![CardType::Sorcery]` is correct.
- **`move_spell_after_resolve` vs. `move_object(Zone::Graveyard)`**: PASS (for the single and empty cases). Both code paths for nonzero/zero nonland cards use `move_spell_after_resolve`, which correctly sends Night Terrors to exile if cast with flashback or graveyard otherwise.

### Test coverage

- **Single nonland card exiled correctly**: `mtg-engine/tests/tier11_cards.rs:308` (`night_terrors_exiles_nonland_from_hand`) — TESTED
- **Land in hand is not exiled (all-lands case)**: `mtg-engine/tests/tier11_cards.rs:323` (`night_terrors_skips_lands`) — TESTED
- **Multiple nonland cards in hand — choice presented and Night Terrors resolves off stack**: NOT TESTED (this is the scenario that exposes the stuck-on-stack bug)
- **Targeting self**: NOT TESTED
- **Controller (not target player) makes the choice when multiple nonland cards are present**: NOT TESTED
