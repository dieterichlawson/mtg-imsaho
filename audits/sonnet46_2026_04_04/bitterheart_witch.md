## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Deathtouch
When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
**Type line**: Creature — Human Shaman
**Status**: ISSUE

### Code issues

- `present_player_choice` builds target list without hexproof filtering (`mtg-engine/src/cards/isd/bitterheart_witch.rs:14-16`) and the corresponding `ChooseCurseThenAttach` path in the engine has the same gap (`mtg-engine/src/engine.rs:2581-2583`).
  - Oracle text says: `"put it onto the battlefield attached to target player"` — "target player" means the player must be a legal target. The Scryfall ruling states: `"The Curse must be legally able to enchant the player. For example, if the player has protection from red, you couldn't put a red Curse onto the battlefield this way."` MTG rules additionally require that a player with hexproof cannot be targeted by an opponent's abilities.
  - Code does: `let player_targets: Vec<crate::actions::Target> = (0..state.players.len()).map(|i| crate::actions::Target::Player(PlayerId(i as u8))).collect();` — all players unconditionally. Neither `can_target_player` (which checks hexproof via `player_has_hexproof`) nor any protection check is called before populating the options list. A player controlling Witchbane Orb (hexproof) can be illegally offered as a target by an opponent's Bitterheart Witch trigger, and the `AttachCurseToPlayer` effect (`engine.rs:2598-2614`) similarly performs no legality check before moving the Curse to the battlefield and setting `attached_to_player`.

### Tricky interactions checked

- **"you may" — yes/no choice presented to controller**: PASS. `on_dies` sets `awaiting_action = Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::YesNo { ... } })` (bitterheart_witch.rs:65-72). The `on_yes_no_choice` hook returns early with no effect when `yes == false` (line 76-78). ✓
- **Controller lookup after death (witch is in graveyard at trigger resolution time)**: PASS. `get_object` in state.rs is an unconditional HashMap lookup (`self.objects.get(&id)`, state.rs:578-580) — no zone filter. The witch object persists in the HashMap after moving to Zone::Graveyard, so `state.get_object(object_id).map(|o| o.controller)` succeeds in both `on_dies` and `on_yes_no_choice`. ✓
- **SelfDies trigger fires from graveyard correctly**: PASS. `collect_triggers` captures `dead_id`, `dead_card_id`, and `controller` from the `CreatureDied` event before zone change (triggers.rs:394-415). `resolve_next_trigger` calls `behavior.on_dies(state, dead_id, registry)` with no zone guard on the dead creature — correct for a dies trigger. ✓
- **"target player" includes self (controller can curse themselves)**: PASS. The player list `(0..state.players.len())` includes all players including the controller. Test `bitterheart_witch_can_attach_curse_to_self` confirms P0 can attach to P0. ✓
- **Shuffle after attach (not before)**: PASS. In `AttachCurseToPlayer` (engine.rs:2598-2613) the shuffle occurs after `state.move_object` and `obj.attached_to_player = Some(*pid)`, matching "then shuffle" in oracle text. ✓
- **Shuffle even when no Curse found**: PASS. Lines 93-101 in bitterheart_witch.rs log "no Curse found" and still call `library_order.shuffle`, consistent with MTG rule CR 701.19c (library is still shuffled even if nothing is found). ✓
- **Multiple Curses in library — player choice presented**: PASS. When `curse_ids.len() > 1`, a `ChooseTarget` with all Curse object IDs is presented (lines 108-125), then `ChooseCurseThenAttach` in engine.rs presents the player target choice. ✓
- **Single Curse in library — auto-selected, then player target chosen**: PASS (functionally). When `curse_ids.len() == 1`, the single Curse is auto-selected and the player target choice is presented immediately (lines 103-106). This bypasses the "choose not to find" option (CR 701.19c), but that scenario has no practical impact in normal gameplay and is not flagged.
- **Summoning sickness cleared for Curse entering battlefield**: PASS. `move_object` sets `summoning_sick = true` for any object entering the battlefield (state.rs:490-492); `AttachCurseToPlayer` immediately overrides with `obj.summoning_sick = false` (engine.rs:2606). Enchantments don't have summoning sickness. ✓
- **ETB event emitted for Curse entering battlefield**: PASS. `move_object` emits `GameEvent::EnteredBattlefield` whenever an object transitions to `Zone::Battlefield` (state.rs:503-514). This allows any Curse ETB abilities to fire via `collect_triggers`. ✓
- **Curse subtype check for library search (registry-only check)**: PASS. Library objects are registry-backed cards, not tokens; their subtypes are not modified at runtime. Checking `registry.card_data(card_id).map(|d| d.subtypes.iter().any(|s| s == "Curse"))` (bitterheart_witch.rs:86-89) is sufficient for this context. ✓
- **Hexproof player targetable by opponent via trigger resolution**: FAIL. See Code Issues above. `present_player_choice` and `ChooseCurseThenAttach` both build player target lists without calling `can_target_player` (engine.rs:772-776) or `player_has_hexproof` (state.rs:1144-1152). ✓ engine has the infrastructure but it is not used here.
- **Protection from Curse's color (Scryfall ruling)**: FAIL (part of same issue above). `AttachCurseToPlayer` does no color-protection check before attaching the Curse. The ruling is unimplemented.

### Test coverage

- "you may" yes/no choice: `mtg-engine/tests/tier15_cards.rs:176` (`bitterheart_witch_finds_curse_on_death`) and `mtg-engine/tests/tier15_cards.rs:252` (`bitterheart_witch_decline_search`) ✓
- Curse found and attached to opponent: `mtg-engine/tests/tier15_cards.rs:176` ✓
- Curse attached to self: `mtg-engine/tests/tier15_cards.rs:217` (`bitterheart_witch_can_attach_curse_to_self`) ✓
- Decline search, Curse remains in library: `mtg-engine/tests/tier15_cards.rs:252` ✓
- Hexproof player cannot be targeted by Bitterheart Witch trigger: NOT TESTED
- Protection from Curse's color prevents attachment (Scryfall ruling): NOT TESTED
- Multiple Curses in library (player chooses which): NOT TESTED
- No Curse in library (still shuffles): NOT TESTED
