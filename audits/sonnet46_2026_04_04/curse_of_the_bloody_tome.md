## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, that player mills two cards.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Trigger fires on both players' upkeeps (engine design)**: The engine's `collect_triggers` (triggers.rs:597–642) fires ALL `TriggerKind::Upkeep` triggers at `StepStarted { Upkeep }` regardless of whose upkeep it is. The card's `on_upkeep` (curse_of_the_bloody_tome.rs:56–58) guards with `if state.active_player != cursed_player { return; }`. Critically, `process_triggers` resolves all triggers synchronously before players ever receive priority, so the no-op trigger during the non-enchanted player's upkeep is never visible on the stack. Correct effect is produced. PASS
- **Trigger only mills during enchanted player's upkeep**: `on_upkeep` reads `state.active_player` and compares to `o.attached_to_player` from the curse object. Correctly mills during the cursed player's upkeep and skips during the curse controller's own upkeep when those differ. PASS
- **"That player" mills (not the curse controller)**: `mill_cards(state, cursed_player, 2)` uses `cursed_player` derived from `attached_to_player`, not `o.controller`. PASS
- **Fewer than 2 cards in library (ruling: put only remaining card into graveyard)**: `mill_cards` (engine.rs:2755–2771) loops with `break` when `library_order.is_empty()`, so milling stops at actual library size. PASS
- **Hexproof player cannot be targeted by Curse**: `TargetRequirement::PlayerOnly` generates legal actions via `can_target_player` (engine.rs:772–777) which calls `state.player_has_hexproof`. If the opponent has hexproof (e.g., Witchbane Orb), the Curse cannot be cast targeting them. PASS
- **Curse survives when enchanted player gains hexproof after attachment**: No SBA removes a player-attached aura (sba.rs:152–166 only checks `attached_to.is_some()`, which is false for curses using `attached_to_player`). The curse persists unless explicitly destroyed. PASS
- **Witchbane Orb ETB destroys already-attached curse**: witchbane_orb.rs correctly finds curses via `attached_to_player == Some(controller)` and destroys them. If destroyed before the upkeep trigger resolves (within `process_triggers`), `resolve_next_trigger` checks `zone == Battlefield` before calling `on_upkeep` and skips the effect. PASS
- **Trigger not fired on opponent's upkeep (no spurious milling)**: Confirmed no milling occurs when `state.active_player != cursed_player`. PASS
- **Log accuracy when library has fewer than 2 cards**: Minor issue — the log at curse_of_the_bloody_tome.rs:60–61 unconditionally prints "milled 2 cards" even when `mill_cards` milled fewer. `mill_cards` itself logs accurately. This is a logging inaccuracy only, with no game-state impact.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Trigger mills enchanted player's 2 cards at start of their upkeep: NOT TESTED
- Trigger does not fire during the non-enchanted player's upkeep (no milling on wrong upkeep): NOT TESTED
- Fewer than 2 cards in library — only available cards are milled (Scryfall ruling 2011-09-22): NOT TESTED
- Hexproof player cannot be targeted by the Curse: `witchbane_orb.rs` tests player hexproof blocking targeting in general, but not specifically for Curse casting — INDIRECTLY TESTED
- Witchbane Orb entering the battlefield destroys an already-attached Curse: NOT TESTED
- Curse attaches to player correctly via resolve_curse: NOT TESTED
