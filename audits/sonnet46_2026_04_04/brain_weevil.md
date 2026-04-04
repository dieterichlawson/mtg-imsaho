## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
**Type line**: Creature — Insect
**Status**: ISSUE

### Code issues

- Incomplete discard when target player has 3+ cards in hand — `mtg-engine/src/cards/isd/brain_weevil.rs:64-75` + `mtg-engine/src/engine.rs:2009-2023`
  - Oracle text says: `Target player discards two cards.`
  - Code does: When the target player has 3 or more cards in hand, `on_activate_ability` sets up a single `ChooseCardFromHand` prompt (described as "1 of 2"), but `on_discard_choice` is never implemented on `BrainWeevil`. The engine calls `behavior.on_discard_choice(...)` after the first chosen card is moved to the graveyard (engine.rs:2022), but the default no-op fires and no second discard choice is ever set up. Result: the target player discards only 1 card instead of 2 when they have 3 or more cards.

  Concretely, engine.rs lines 2009–2023 handle `ChooseCardFromHand` resolution:
  ```
  behavior.on_discard_choice(&mut new_state, choice_source, *discard_id, registry);
  ```
  And brain_weevil.rs has no `on_discard_choice` implementation (uses the default no-op in `CardBehavior`), so no second `ChooseCardFromHand` is ever queued.

### Tricky interactions checked

- **Sorcery-speed-only restriction**: `sorcery_speed_only: true` is set on the `ActivatedAbilityDef` and the engine enforces it with `if ab.sorcery_speed_only && !is_sorcery_speed { continue; }` (engine.rs:360). `is_sorcery_speed` is correctly defined as main phase + empty stack + your turn (engine.rs:302–304). PASS.
- **Sacrifice-as-cost ordering**: `SacrificeCost::SacrificeThis` is paid before `on_activate_ability` is called (engine.rs:1747–1748). Brain Weevil is moved to graveyard before the discard effect fires. `get_object(choice_source)` in the `ResolveChoice` handler still finds it (objects stay in `self.objects` across zone changes, only the `zone` field changes). The `on_discard_choice` callback is reachable; the bug is that it does nothing. PASS (sacrifice timing is correct; the callback lookup works).
- **Target is any player (including self)**: `TargetRequirement::PlayerOnly` allows targeting any player, not just opponents. The oracle text says "Target player" with no restriction. This is correct. PASS.
- **Discard when target has 0 cards**: `hand.is_empty()` returns early without pushing a `Discarded` event. Per MTG rules, "discard two cards" with 0 in hand means discard 0 — no event is the correct outcome. PASS.
- **Discard when target has 1 or 2 cards**: The `hand.len() <= 2` branch discards all cards in a loop (no choice needed). For 1 card, discards 1; for 2 cards, discards both. Correct per MTG rules. PASS.
- **Discard when target has 3+ cards**: Only 1 card is discarded — the first chosen card. No second discard is set up because `on_discard_choice` is not implemented. FAIL (see Code Issues above).
- **Intimidate keyword**: `keywords: vec![Keyword::Intimidate]` is declared. PASS.
- **Card data (mana cost, types, P/T)**: `{3}{B}` = `[Generic(3), Colored(Black)]`, Creature, Insect, 1/1 — all match oracle. PASS.

### Test coverage

- **Sorcery-speed-only restriction**: NOT TESTED
- **Sacrifice is paid as cost (weevil ends up in graveyard)**: `tier8_cards.rs:93` (`brain_weevil_forces_discard`) — TESTED
- **Discard with 0 cards in hand**: NOT TESTED
- **Discard with 1 card in hand**: NOT TESTED
- **Discard with exactly 2 cards in hand (auto-discard path)**: `tier8_cards.rs:93` (`brain_weevil_forces_discard`) — TESTED
- **Discard with 3+ cards in hand (choice path, second discard)**: NOT TESTED — this is the path that contains the bug
- **Intimidate keyword present**: `tier8_cards.rs:129` (`brain_weevil_has_intimidate`) — TESTED
- **Target can be any player (self-targeting)**: NOT TESTED
