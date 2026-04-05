## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, return a Zombie card at random from your graveyard to your hand.
**Type line**: Creature — Zombie
**Status**: ISSUE

### Code issues
- Incorrect early return when source leaves battlefield (`ghoulraiser.rs:42-45`)
  - Oracle text says: `When this creature enters, return a Zombie card at random from your graveyard to your hand.`  
  - Code does: `match state.get_object(object_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, }` — This causes the ETB ability to do nothing if Ghoulraiser leaves the battlefield between trigger and resolution, but per MTG rules, ETB triggers should resolve independently of their source once on the stack.

### Tricky interactions checked
- **Zombie cards vs zombie tokens**: PASS — Code correctly filters only `registry.card_data()` and excludes tokens, which is correct since the oracle text says "Zombie card" and tokens are not cards per MTG rules.
- **Random selection implementation**: PASS — Uses proper `rand::thread_rng()` and `SliceRandom::shuffle()` to randomize the selection.
- **Source leaving battlefield**: FAIL — Code incorrectly returns early if the Ghoulraiser is no longer on the battlefield when the trigger resolves. Per MTG rules, ETB triggers should resolve independently of their source once on the stack.
- **Empty graveyard handling**: PASS — Code properly handles the case where no Zombie cards are in the graveyard by checking `!zombies.is_empty()` before proceeding.
- **EnteredBattlefield trigger generation**: PASS — Verified that `move_object` to Zone::Battlefield generates the `GameEvent::EnteredBattlefield` event which gets collected as a `PendingTrigger::EnteredBattlefield`.
- **Trigger resolution**: PASS — Verified that the trigger system calls `on_enter_battlefield` when resolving `EnteredBattlefield` triggers.

### Test coverage
- **Basic ETB ability**: `mtg-engine/tests/tier11_cards.rs:146` - Tests that Ghoulraiser returns a Zombie creature from graveyard to hand
- **Random selection**: NOT TESTED - No test verifies randomness or multiple Zombies in graveyard
- **Empty graveyard**: NOT TESTED - No test verifies behavior when no Zombies are in graveyard
- **Non-creature Zombie cards**: NOT TESTED - No test verifies that non-creature cards with Zombie subtype can be returned
- **Zombie tokens exclusion**: NOT TESTED - No test verifies that zombie tokens in graveyard are not returned
- **Source leaves battlefield**: NOT TESTED - No test verifies behavior if Ghoulraiser leaves battlefield between trigger and resolution