## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature can't be blocked by Humans.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
**Type line**: Creature — Vampire Noble
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Human cards (non-token) can't block**: `matches_filter` for `HasSubtype("Human")` checks `registry.card_data(creature.card_id)` first, which correctly finds "Human" in the registered subtypes for cards like Unruly Mob. `can_block_attacker` correctly returns false for such blockers: pass
- **Human tokens can't block**: `matches_filter` falls through to check `creature.subtypes` on the object when `registry.card_data` returns `None` (for tokens with `card_id = CardId(0)`). Human tokens with `"Human"` in their `subtypes` field are correctly identified and blocked from blocking: pass
- **Non-Humans can block freely**: `CreatureFilter::Not(HasSubtype("Human"))` evaluates to true for non-Humans, so `can_block_attacker` returns true: pass
- **Counter added only on combat damage to player, not creature**: Trigger uses `TriggerKind::CombatDamageToPlayer`, which is only fired from `GameEvent::CombatDamageDealt` with a `DamageTarget::Player` target, not `DamageTarget::Object`: pass
- **Trigger doesn't fire when Noble is blocked**: A blocked creature with no trample can only deal `CombatDamageDealt` to an `Object` target, never `Player`. Noble's trigger only fires on unblocked hits (or trample excess): pass
- **Counter not added if Noble leaves battlefield before trigger resolves**: The card's `on_combat_damage_to_player` checks `state.get_object(self_id).map(|o| o.zone == Zone::Battlefield)` before calling `add_counters`. If Noble is gone, the zone check fails and no counter is added: pass
- **Trigger resolution path (no zone check at resolve time)**: `resolve_next_trigger` for `CombatDamageToPlayer` does not check the source's zone before calling the behavior. The effect check is inside the card handler (`on_combat_damage_to_player`), which is the correct place: pass
- **First-strike edge case (trigger collection zone check)**: In `collect_triggers`, the `CombatDamageDealt` handler checks `obj.zone == Zone::Battlefield` at collection time. If Noble somehow had first strike + trample, dealt trample damage to a player in the first-strike step, then died from deathtouch SBAs before normal damage, the trigger would not be collected. However, for Stromkirk Noble (a 1/1 with no inherent keywords), this scenario requires externally-granted first strike AND trample AND a deathtouch blocker, and the practical outcome is identical (no counter can be placed on a creature that has already left the battlefield): pass (no practical impact)
- **Block restriction enforced at blocker declaration**: `declare_blockers_with_registry` in `combat.rs:63` calls `can_block_attacker` for each proposed blocker/attacker pair and silently filters out illegal assignments: pass
- **`BlockRestriction` scope is `OnSelf`**: `effect_applies_to(attacker_id, OnSelf, source.id, ...)` checks `attacker_id == source.id`. When iterating over battlefield permanents, only when the source is the Noble itself does the check pass (attacker == noble, source == noble): pass
- **Mana cost, types, P/T, subtypes**: `{R}`, Creature, Vampire/Noble, 1/1 all match oracle: pass

### Test coverage
- Combat damage to player triggers +1/+1 counter: `tier6_cards.rs:152` (`stromkirk_noble_counter_on_combat_damage`)
- Human (real card) can't block Noble: `tier6_cards.rs:173` (`stromkirk_noble_cant_be_blocked_by_humans`)
- Non-Human can block Noble: `tier6_cards.rs:173` (`stromkirk_noble_cant_be_blocked_by_humans`, second assertion)
- Human TOKEN can't block Noble: NOT TESTED
- Counter not added if Noble leaves battlefield before trigger resolves: NOT TESTED
- First-strike trample + deathtouch edge case (trigger-collection zone check): NOT TESTED
