## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard. Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Fewer than 4 cards in library**: `std::cmp::min(4, player.library_order.len())` at line 36 correctly caps the reveal at however many cards remain. Oracle ruling confirmed: "If you have fewer than four cards in your library, you'll look at all the cards there" — handled correctly.
- **Zero cards in library**: `revealed.is_empty()` check at line 39 causes the spell to resolve with no effect, then calls `move_spell_after_resolve`. Correct (nothing to put in hand or graveyard).
- **Exactly one card revealed**: Line 42–48 auto-places the single revealed card in hand without presenting a choice. Correct — there is no choice to make when only one card is available.
- **Player must choose (mandatory, not "may")**: Oracle text says "Put one of them into your hand" — no "you may." The `ChooseFromRevealed` choice set in the engine (lines 215–219 of engine.rs) always generates one action per revealed card; the player cannot decline. Correct.
- **Flashback exile**: `move_spell_after_resolve` (state.rs:1132–1141) checks `obj.cast_with_flashback`. This flag is set at engine.rs:1637 during `CastSpell` when the card is being cast from graveyard (`is_flashback = in_graveyard && !is_cast_from_graveyard` at line 1492). When the resolution choice is submitted, the handler at engine.rs:2035 calls `move_spell_after_resolve(*spell_id)`, which exiles the card. Correct.
- **spell_id in ChooseFromRevealed resolves correctly**: The `spell_id` stored in `ChooseFromRevealed` (lines 61–62 of the card file) is the Forbidden Alchemy object id. The `cast_with_flashback` flag set at cast-time persists on that object through the awaiting-choice interval and is still present when `move_spell_after_resolve` runs. Correct.
- **All revealed cards available as choices**: engine.rs:215–219 maps every entry in `revealed` to a `ResolvedChoice::ChosenCard(id)` action with no filtering. Correct.
- **Non-chosen cards all go to graveyard**: engine.rs:2029–2032 iterates all `revealed` cards and moves any that are not `keep_id` to `Zone::Graveyard`. Correct.
- **Flashback keyword not in `keywords` vec**: Scryfall lists "Flashback" in its Keywords field but the engine's `Keyword` enum only covers keyword abilities (Flying, Trample, etc.); flashback is handled via the dedicated `flashback_cost` field. The empty `keywords: vec![]` is correct.
- **Mana costs**: Normal cost `{2}{U}` → `Generic(2), Colored(Blue)` ✓. Flashback cost `{6}{B}` → `Generic(6), Colored(Black)` ✓.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Choose 1 of 4 revealed cards; chosen card to hand, others to graveyard: `card_mechanics.rs:724` (4-card library, pick one, verify all zones)
- Basic mechanics with library > 4 (reveal top 4 of 5, 1 to hand, 3 to graveyard): `flashback.rs:380`
- Fewer than 4 cards in library (min-clamp logic): NOT TESTED
- Zero cards in library: NOT TESTED
- Exactly one card in library (auto-put in hand): NOT TESTED
- Forbidden Alchemy cast via flashback → exiled after resolution: NOT TESTED (general flashback exile is tested for other cards such as Geistflame/Think Twice/Bump in the Night; no Forbidden Alchemy–specific flashback cast test exists)
