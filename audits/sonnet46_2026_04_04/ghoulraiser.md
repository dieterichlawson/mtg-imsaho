## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, return a Zombie card at random from your graveyard to your hand.
**Type line**: Creature — Zombie
**Status**: ISSUE

### Code issues

- ETB trigger silently skipped if Ghoulraiser leaves the battlefield before the trigger resolves — engine bug in `mtg-engine/src/triggers.rs` lines 893–899
  - Oracle text says: `When this creature enters, return a Zombie card at random from your graveyard to your hand.`
  - Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { if let Some(behavior) = registry.get(card_id) { behavior.on_enter_battlefield(state, object_id, registry); } }` — the entire effect is skipped if Ghoulraiser is no longer on the battlefield (e.g., destroyed or bounced in response to the trigger). Per MTG rules (CR 603.10), the triggered ability is already on the stack and resolves independently of the source's continued presence; the effect "return a Zombie card at random from your graveyard to your hand" does not require Ghoulraiser to still be on the battlefield.

### Tricky interactions checked

- **Mandatory vs optional**: Oracle says "return a Zombie card at random" (no "you may") — code makes the effect mandatory with no player choice presented. PASS
- **"at random" selection**: Code shuffles the candidate list with `zombies.shuffle(&mut rng)` then takes `zombies[0]`. Correctly random. PASS
- **"your graveyard"**: Code filters by `state.objects_in_zone(Zone::Graveyard, controller)` where `controller` is the Ghoulraiser controller at trigger collection time. PASS
- **"to your hand"**: Code calls `state.move_object(chosen, Zone::Hand)`. Correct destination. PASS
- **"Zombie card" — tokens excluded**: The subtype check only consults `registry.card_data(o.card_id)`. Zombie tokens have `card_id: CardId(0)` and are not registered, so they return `unwrap_or(false)`. However, tokens are not "cards" per MTG rules and cease to exist when they leave the battlefield, so they can never be in the graveyard to be selected. The registry-only check is correct for this wording. PASS
- **"Zombie card" — any card type with Zombie subtype**: Oracle says "Zombie card" not "Zombie creature card". The code checks `d.subtypes.iter().any(|s| s == "Zombie")` with no card-type restriction, which correctly includes non-creature Zombie cards (Zombie Artifacts, Zombie Enchantments, etc.) if they were in the graveyard. PASS
- **Source leaves battlefield before trigger resolves**: If Ghoulraiser is removed from the battlefield (bounced, destroyed, exiled) in response to its ETB trigger, the engine's `resolve_next_trigger` checks `o.zone == Zone::Battlefield` and skips the effect entirely. Per MTG rules, the trigger is already on the stack and should resolve regardless of the source's location. FAIL (see Code Issues)
- **No Zombie cards in graveyard**: Code checks `if !zombies.is_empty()` before acting; if no Zombie cards are present the trigger resolves with no effect. Correct — the oracle text says "return a Zombie card" implying the effect does nothing if none exist. PASS
- **ETB trigger kind declared**: `triggered_abilities` includes `TriggerKind::EntersBattlefield` with description `"return a random Zombie from graveyard to hand"`. The engine's `collect_triggers` dispatches `EnteredBattlefield` events and resolves them via `on_enter_battlefield`. PASS

### Test coverage

- Basic ETB returning one Zombie from graveyard to hand: `mtg-engine/tests/tier11_cards.rs:146` (`ghoulraiser_returns_zombie_from_graveyard`) — TESTED
- No Zombie card in graveyard (trigger resolves with no effect): NOT TESTED
- Multiple Zombie cards in graveyard (only one returned at random): NOT TESTED
- Ghoulraiser removed from battlefield before trigger resolves (effect should still happen per MTG rules): NOT TESTED
- Zombie card that is not a creature (e.g., Zombie Enchantment) is eligible: NOT TESTED
