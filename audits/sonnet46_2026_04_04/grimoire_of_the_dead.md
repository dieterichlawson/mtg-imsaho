## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
**Type line**: Legendary Artifact
**Status**: ISSUE

### Code issues

- Legend rule not applied to legendary creature cards returned by ability 2 if they were never previously on the battlefield (`mtg-engine/src/cards/isd/grimoire_of_the_dead.rs:151-163`, `mtg-engine/src/sba.rs:290`)
  - Oracle text says: `Put all creature cards from all graveyards onto the battlefield under your control.`
  - Code does: `state.move_object(cid, Zone::Battlefield)` moves the creature to the battlefield, but `is_legendary` is only set to `true` in `on_resolve` — called only when a card resolves from the stack. Cards milled or discarded directly from the hand/library to the graveyard (never cast) are created with `is_legendary: false` (default in `state.rs:278`) and never have `on_resolve` called. The SBA legend-rule check at `sba.rs:290` is `if obj.zone == Zone::Battlefield && obj.is_legendary`, so these returned legendary creatures with `is_legendary = false` are excluded from the legend group, and the legend rule never fires for them. Two copies of the same legendary creature could co-exist on the battlefield (e.g., one cast and killed, one milled, both returned by Grimoire).

### Tricky interactions checked

- **Ability 2 fires after Grimoire is sacrificed**: The engine finds the ability before sacrifice occurs (line 1703–1715 in `engine.rs`), then pays the sacrifice cost (line 1747–1749). The `behavior_card_id` lookup after sacrifice falls back to `card_id` (since `activated_abilities` returns empty for a graveyard object), and `on_activate_ability` is called correctly. Pass.
- **Study counters not explicitly removed**: The oracle requires "Remove three study counters" as a cost, but the engine has no counter-removal mechanism for activated ability costs. Only the check `study_counters >= 3` gates the ability's availability. Because the Grimoire is also sacrificed in the same cost and moves to the graveyard taking its counters with it, the functional outcome is identical. Pass (moot given simultaneous sacrifice).
- **"All graveyards" includes both players**: `state.objects.values()` iterates all game objects with no player filter; `o.zone == Zone::Graveyard` is the only zone check. Correctly covers all graveyards. Pass.
- **"Under your control" assignment**: `obj.controller = controller` is set for each returned creature, where `controller` is fetched from the Grimoire's graveyard object before the loop. Pass.
- **"Black Zombies in addition to their other colors and types"**: Code adds `Color::Black` and `"Zombie"` to `obj.colors` and `obj.subtypes` only if not already present. Pass.
- **Grimoire excluded from its own creature sweep**: Filter `o.id != object_id` explicitly excludes the Grimoire. Additionally Grimoire has `power = None`, so even without this guard it would be excluded by the creature filter. Pass.
- **ETB triggers for returned creatures**: `move_object` pushes `EnteredBattlefield` event for each creature. `run_game` calls `triggers::process_triggers` after each action, which fires `on_enter_battlefield` for all returned creatures. Pass.
- **Summoning sickness on returned creatures**: `move_object` sets `summoning_sick = true` when entering the battlefield from any other zone. Pass.
- **No "may" clause — mandatory return**: The oracle text has no "may." All matching creatures are returned unconditionally. Code collects all matching objects and iterates the full set with no player opt-out. Pass.
- **Discard-as-cost timing for ability 1 (single card in hand)**: Auto-discards immediately within `on_activate_ability`, before adding the study counter. Order is correct. Pass.
- **Discard-as-cost timing for ability 1 (multiple cards in hand)**: Sets `AwaitingAction::ResolutionChoice` with `ChooseCardFromHand`. Player picks a card; engine discards it, pushes `Discarded` event, then calls `on_discard_choice` which adds the study counter. Order is correct. Pass.
- **Legend rule for legendary creatures previously on battlefield (died and returned)**: When a legendary creature dies normally, `move_object` does NOT reset `is_legendary`. So `is_legendary = true` persists in the graveyard. When Grimoire returns it, `is_legendary` is still `true` and the SBA legend-rule check fires correctly. Pass.
- **Legend rule for legendary creatures never on battlefield (milled/discarded)**: FAIL — see Code Issues above.
- **`ability_index` uniqueness when not all abilities are present**: Ability 0 is omitted when the controller has no cards in hand; ability 1 is omitted when < 3 study counters. The engine finds abilities by `ability_index` field, not Vec position, so the correct ability is always found. Pass.

### Test coverage

- Discard choice presented to player (multiple cards in hand): `tier15_cards.rs:2585` — TESTED
- Auto-discard with single card in hand: `tier15_cards.rs:2626` — TESTED
- Study counter accumulates to 3: `tier15_cards.rs:2656` — TESTED
- Ability 2 returns all graveyard creatures as black Zombies under controller's control: `tier15_cards.rs:2699` — TESTED
- Ability 2 unavailable with fewer than 3 study counters: `tier15_cards.rs:2740` — TESTED
- Grimoire sacrificed after ability 2: `tier15_cards.rs:2735` — TESTED
- Legend rule interaction with returned legendary creatures: NOT TESTED
- Ruling 2011-09-22 (artifact creatures and other multi-type creatures in graveyard are included): NOT TESTED
