## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
**Type line**: Enchantment — Aura Curse
**Status**: ISSUE

### Code issues

- Upkeep trigger fires during every player's upkeep, not only the enchanted player's upkeep — spurious triggers placed on stack during the non-cursed player's turn.
  - Oracle text says: `"At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard."`
  - Code does: In `triggers.rs` `collect_triggers()`, the `GameEvent::StepStarted { step: Upkeep }` branch (lines 597–643) iterates over ALL battlefield permanents with a non-empty `TriggerKind::Upkeep` description and pushes an `UpkeepTrigger` to the stack every upkeep step. The Curse of Oblivion's description is `"enchanted player exiles two cards from their graveyard"` (non-empty), so the trigger goes on the stack at the beginning of EVERY player's upkeep — including the non-cursed player's. The `on_upkeep` handler (`curse_of_oblivion.rs:57`) guards with `if state.active_player != cursed_player { return; }`, meaning no exile occurs during the wrong upkeep, but the trigger still appears on the stack and players receive priority around it at the wrong time. In real MTG, "At the beginning of enchanted player's upkeep" means the trigger should only go on the stack at the beginning of that specific player's upkeep.

### Tricky interactions checked

- **Mandatory vs. optional exile**: Oracle text says "that player exiles" (mandatory). Code sets `optional: false` in `ChooseTarget` choice — PASS.
- **Ruling: only 1 card in graveyard → exile that card**: Code path at `curse_of_oblivion.rs:70–80` — `if gy_cards.len() <= 2 { exile all; return }` — correctly exiles the single card without presenting a choice — PASS.
- **0 cards in graveyard → no exile**: Code at `curse_of_oblivion.rs:66–68` returns early when `gy_cards.is_empty()` — PASS.
- **2 cards in graveyard → exile both without choice**: The `gy_cards.len() <= 2` branch exiles both without prompting — PASS.
- **3+ cards in graveyard → player must choose 2**: Code presents sequential choices via `PendingEffect::ExileCurseOfOblivion { remaining: 1 }`, then engine handler (`engine.rs:2277–2299`) presents a second choice with `remaining: 0` after the first exile — PASS.
- **Second exile choice uses `source: ObjectId(0)` (dummy)**: `engine.rs:2290` sets `source: crate::ids::ObjectId(0)`. The `choice_source` value is only used in the `ChooseCardFromHand` match arm (`engine.rs:2020`), not in the `ChooseTarget` branch (`engine.rs:2003–2008`), so this dummy ID is harmless — PASS.
- **Trigger only fires when curse is on battlefield**: `on_upkeep` handler checks `Some(o) if o.zone == Zone::Battlefield` (`curse_of_oblivion.rs:50`) before reading `attached_to_player`, and `resolve_next_trigger` also verifies the permanent is on the battlefield before calling `on_upkeep` (`triggers.rs:955`) — PASS.
- **Curse attachment to target player**: `on_resolve` delegates to `helpers::resolve_curse` which sets `obj.attached_to_player = Some(*player_id)` and moves to battlefield — PASS.
- **TargetRequirement::PlayerOnly for "Enchant player"**: `target_requirement()` returns `TargetRequirement::PlayerOnly` — PASS.
- **Spurious trigger during non-cursed player's upkeep**: As described above, the trigger fires every upkeep and resolves with no effect outside the cursed player's turn, but incorrectly occupies the stack — ISSUE (see Code issues).
- **`objects_in_zone` graveyard scope**: Uses `obj.owner == player` filter for graveyard zone, matching cards owned by (and therefore in the graveyard of) the cursed player — PASS.
- **Card data matches oracle**: mana cost `{3}{B}`, card type `Enchantment`, subtypes `["Aura", "Curse"]`, oracle_text verbatim match — PASS.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- 2-card graveyard (exile both automatically): `mtg-engine/tests/tier7_cards.rs:299` (`curse_of_oblivion_exiles_from_graveyard`) — TESTED
- 1-card graveyard ruling (exile that card): NOT TESTED
- 0-card graveyard (do nothing): NOT TESTED
- 3+ card graveyard (player chooses 2): NOT TESTED
- Spurious trigger during non-cursed player's upkeep: NOT TESTED
- Curse correctly not firing during owner's (non-cursed player's) upkeep: NOT TESTED
- `resolve_curse` attaches to correct player: NOT TESTED (for this card specifically; pattern tested by other curse tests)
