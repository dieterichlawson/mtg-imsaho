## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
At the beginning of your upkeep, target player draws a card and loses 1 life.
**Type line**: Creature — Demon
**Status**: ISSUE

### Code issues

- Upkeep trigger incorrectly fizzles if Bloodgift Demon leaves the battlefield after its trigger is on the stack but before it resolves.
  - Oracle text says: `"At the beginning of your upkeep, target player draws a card and loses 1 life."`
  - Code does: In `mtg-engine/src/triggers.rs` lines 954–959, `resolve_next_trigger` wraps the entire `UpkeepTrigger` resolution in a battlefield check: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)`. If Bloodgift Demon is no longer on the battlefield when the trigger resolves (e.g., because another upkeep trigger in the same batch killed it first — NAP kills Demon, then AP's Demon trigger tries to resolve), the entire ability is silently skipped. A second guard in `bloodgift_demon.rs` line 41 (`Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return`) also fizzles the ability for the same reason. Per MTG rules, a triggered ability that has been placed on the stack resolves regardless of whether its source has since left the battlefield. The ability "target player draws a card and loses 1 life" has no zone dependency on the source and must resolve even if the Demon has died.

### Tricky interactions checked

- **"at the beginning of YOUR upkeep" — trigger scoped to controller's upkeep**: The upkeep trigger is dispatched for ALL permanents on the battlefield at every upkeep step (triggers.rs line 604–640 has no `controller == active_player` filter). During an opponent's upkeep, Bloodgift Demon's trigger still goes on the stack. The guard in `on_upkeep` (`if state.active_player != controller { return; }`) prevents the effect from firing at the wrong time. Since the engine processes triggers synchronously without priority windows, the spurious trigger fizzles invisibly. Net game-outcome is correct, but technically the trigger should not go on the stack at all during an opponent's upkeep. Pass (correct outcome).
- **"target player" includes self**: `on_upkeep` builds the target list from all non-lost players (`state.players.iter().filter(|p| !p.lost)`), including the controller. This matches "target player" (any player). Pass.
- **"may" optionality — mandatory target**: `optional: false` in the `ChooseTarget` choice. The oracle text has no "you may," so the controller must choose a target. Pass.
- **Draw-then-lose-life sequencing**: `draw_cards` is called first, then life is decremented. Per MTG rules "and" resolves left-to-right: draw first, then lose life. Pass.
- **Life loss vs. damage**: Life is directly decremented (`new_life = old - 1`, engine.rs line 2267–2269) and a `LifeChanged` event is emitted. This correctly models "loses 1 life" (life loss) rather than damage. Pass.
- **Source leaves battlefield before trigger resolves (e.g., another upkeep trigger kills Demon first)**: FAIL — see Code issues above. In APNAP LIFO order, if an opponent's upkeep trigger resolves first and kills Bloodgift Demon, the Demon's own trigger subsequently fizzles due to battlefield checks in both `resolve_next_trigger` (triggers.rs ~955) and `on_upkeep` (bloodgift_demon.rs line 41). Per rules, the trigger should resolve and the target should draw and lose life.
- **Mana cost {3}{B}{B}**: Encoded as `Generic(3), Colored(Black), Colored(Black)`. Pass.
- **P/T 5/4**: `power: Some(5), toughness: Some(4)`. Pass.
- **Flying keyword**: `keywords: vec![Keyword::Flying]`. Pass.
- **Subtype "Demon"**: `subtypes: vec!["Demon".into()]`. Pass.

### Test coverage

- **Basic upkeep draw-and-lose-life**: `mtg-engine/tests/tier7_cards.rs:70` (`bloodgift_demon_draws_and_loses_life`) — covers normal case where controller chooses themselves as target. Tested.
- **Controller chooses opponent as target**: NOT TESTED
- **Trigger fires only during controller's upkeep (not opponent's)**: NOT TESTED
- **Source leaves battlefield after trigger placed on stack**: NOT TESTED
- **Target player has hexproof (should be excluded from options for opponent's ability)**: NOT TESTED
- **Mandatory targeting (no target available, e.g. all players lost)**: NOT TESTED
