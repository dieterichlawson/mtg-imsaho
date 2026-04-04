## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Counter target spell unless its controller pays {1}. That player discards a card.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Discard always happens (ruling: "even if they pay {1}")**: pass. In `engine.rs` lines 1957–1993, the discard block (lines 1971–1992) runs unconditionally after both branches of the `if !*pay { ... } else { ... }` block, so the discard fires whether the opponent pays or not. Confirmed by test `frightful_delusion_discard_on_pay` in `card_fixes.rs`.

- **Auto-counter when opponent has 0 mana**: pass. The card checks `state.get_player(controller).mana_pool.total() >= 1` (line 50). When `can_pay` is false, the spell is countered inline (lines 69–71) without a `PayOrNot` choice, because there is no real decision to make. The discard then proceeds immediately (lines 73–93). This is functionally correct.

- **Spell cleanup (`move_spell_after_resolve`) called exactly once in every path**: pass. Three execution paths exist: (1) `can_pay == false`, 0–1 cards in hand → falls through to line 97; (2) `can_pay == false`, 2+ cards → line 91 calls it before returning; (3) `can_pay == true` → `PayOrNot` deferred to engine, which calls it at line 1990 (2+ card case) or line 1993 (0–1 card case). No path calls it twice.

- **Fizzle if target leaves the stack before resolution**: pass. `stack.rs::is_target_legal` (line 33) for the default target requirement case returns true only for `zone == Battlefield || zone == Stack`. If the targeted spell was countered into the graveyard or exile, the target is illegal, and the fizzle check at `stack.rs` line 81 fires before `on_resolve` is ever called — no discard occurs. This is correct per MTG rules (fizzled spells have no effect).

- **Discard from the correct player (controller of targeted spell)**: pass. The `controller` variable throughout both the card code and the engine `PayOrNot` handler is derived from `state.get_object(*target_id).map(|o| o.controller)`, which is the controller of the targeted spell — "its controller" per the oracle text.

- **`PayOrNot` choice presents both options to the player**: pass. `engine.rs` lines 189–193 expose both `PayDecision(true)` and `PayDecision(false)` as legal actions.

- **Empty hand produces no discard**: pass. Both the card code (lines 73–93 in `on_resolve`) and the engine handler (lines 1973–1992) use `if hand.len() == 1 { ... } else if !hand.is_empty() { ... }`. When hand is empty, neither branch executes and no discard happens — correct per MTG rules.

- **Mana cost and card types**: pass. `{2}{U}` implemented as `ManaSymbol::Generic(2)` + `ManaSymbol::Colored(Color::Blue)`; type is `CardType::Instant` with no subtypes or supertypes.

- **Target requirement restricts to spells on the stack**: pass. `target_requirement()` returns `TargetRequirement::Spell`; `is_valid_target` checks `obj.zone == Zone::Stack`; the fizzle check enforces legality at resolution.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:

- Counter target spell (basic counter path): `mtg-engine/tests/tier2_spells.rs:89` (`frightful_delusion_counters_and_discards`) — TESTED
- "That player discards a card" when spell is countered (1 card in hand, auto-select): `mtg-engine/tests/tier2_spells.rs:89` — TESTED
- "That player discards a card even if they pay {1}" (ruling): `mtg-engine/tests/card_fixes.rs:153` (`frightful_delusion_discard_on_pay`) — TESTED
- PayOrNot choice presented when opponent has mana: `mtg-engine/tests/card_mechanics.rs:576` (`frightful_delusion_choice_when_opponent_has_mana`) — TESTED
- Auto-counter when opponent has no mana: `mtg-engine/tests/card_mechanics.rs:616` (`frightful_delusion_auto_counters_without_mana`) — TESTED
- Discard with 2+ cards in hand requiring ChooseCardFromHand (no-mana branch): NOT TESTED
- Discard with 2+ cards in hand requiring ChooseCardFromHand (pay branch): NOT TESTED
- Fizzle when target leaves stack before resolution: NOT TESTED
- Empty hand produces no discard: NOT TESTED
