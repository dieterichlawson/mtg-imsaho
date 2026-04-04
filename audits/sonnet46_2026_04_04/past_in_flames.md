## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- `until_end_of_turn_flashback` is never cleared at end-of-turn cleanup, so flashback grants persist indefinitely across turns.
  - Oracle text says: `gains flashback until end of turn`
  - Code does: `mtg-engine/src/engine.rs` lines 3020–3025 clear `until_end_of_turn_effects`, `until_end_of_turn_keywords`, `until_end_of_turn_cant_block`, `until_end_of_turn_protection`, and `until_end_of_turn_removed_keywords` at `Step::Cleanup`, but `until_end_of_turn_flashback` is not cleared. Once Past in Flames grants flashback to cards in the graveyard, those cards retain the flashback ability permanently (until cast and exiled, or game ends), even through future turns.

- Cards with no mana cost that are instants or sorceries receive `ManaCost::free()` as their flashback cost, making them castable for {0} when the oracle text and ruling say they cannot be cast via flashback.
  - Oracle text says: `The flashback cost is equal to its mana cost.` — Ruling: `If a card with no mana cost gains flashback, it has no flashback cost. It can't be cast this way.`
  - Code does: `mtg-engine/src/cards/isd/past_in_flames.rs` line 53: `d.cost.clone().unwrap_or(ManaCost::free())` — when `d.cost` is `None` (card has no mana cost), `ManaCost::free()` is used as the granted flashback cost, which the engine treats as a payable {0} cost.

### Tricky interactions checked

- "until end of turn" expiration — FAIL: `until_end_of_turn_flashback` is never cleared in `Step::Cleanup` (`engine.rs` lines 3020–3025 clear five other `until_end_of_turn_*` vecs but omit this one). Flashback grants from Past in Flames persist across turns indefinitely.
- Snapshot: only cards in graveyard at time of resolution gain flashback, not cards added later — PASS: `on_resolve` scans `state.objects` once at resolution time; no continuous re-evaluation.
- Only instants and sorceries gain flashback, not creatures or other card types — PASS: the filter correctly checks `d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery)`.
- "Your" graveyard only (not opponent's) — PASS: filter uses `o.owner == controller` where `controller` is the controller of Past in Flames, correctly targeting owned cards.
- Mana cost of the card becomes the flashback cost — PASS for normal cards: `d.cost.clone()` correctly retrieves the card's mana cost from the registry. Edge case with `None` mana cost noted as a separate issue above.
- Flashback spell is exiled after casting — PASS: engine sets `cast_with_flashback = true`; `move_spell_after_resolve` correctly routes to `Zone::Exile` for those spells.
- Deduplication: a card that already has dynamically-granted flashback doesn't get a duplicate entry — PASS: `on_resolve` checks `state.until_end_of_turn_flashback.iter().any(|(id, _)| *id == target_id)` before pushing.
- Past in Flames itself (if cast normally, ends up in graveyard) not included in own effect — PASS (effectively): the code calls `move_spell_after_resolve` first and then excludes `object_id` from the scan. Practically identical to scanning before moving, and Past in Flames already has printed flashback {4}{R}, so granting it temporary flashback would be redundant anyway.
- Flashback cost correctly used by engine when casting granted-flashback cards — PASS: `engine.rs` lines 1499–1505 look up `until_end_of_turn_flashback` by object ID and use that cost when `is_flashback` is true.
- Past in Flames' own mana cost {3}{R} and flashback cost {4}{R} — PASS: `card_data()` declares `cost: Some(ManaCost::new([Generic(3), Colored(Red)]))` and `flashback_cost: Some(ManaCost::new([Generic(4), Colored(Red)]))`.
- `keywords: vec![]` — PASS: Flashback is not a member of the engine's `Keyword` enum (which covers combat keywords and evasion abilities only); the ability is correctly represented by the dedicated `flashback_cost` field.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Flashback granted to all instants and sorceries in graveyard at resolution: `tests/tier14_cards.rs:427` (`past_in_flames_grants_flashback_to_all`) — TESTED
- Creatures do NOT gain flashback: `tests/tier14_cards.rs:427` — TESTED
- "until end of turn" — flashback expires at cleanup: NOT TESTED (no test advances through the cleanup step and verifies `until_end_of_turn_flashback` is empty afterwards)
- Only cards in graveyard at time of resolution (snapshot, not continuous): NOT TESTED
- "Your" graveyard only: NOT TESTED
- Flashback cost equals the card's mana cost: NOT TESTED explicitly (tested implicitly by the cast affordance in other flashback tests)
- Card with no mana cost cannot be cast via flashback: NOT TESTED
- Past in Flames itself gains flashback from own effect (excluded in code, redundant with printed ability): NOT TESTED
- Flashback spell exiled after resolution: `tests/flashback.rs:86` (`flashback_spell_is_exiled_after_resolve`) — TESTED (for Geistflame; general mechanic)
- Flashback spell countered is still exiled: `tests/flashback.rs:129` (`flashback_spell_countered_is_exiled`) — TESTED (for Geistflame; general mechanic)
