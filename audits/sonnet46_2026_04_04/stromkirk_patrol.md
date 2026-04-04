## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
**Type line**: Creature — Vampire Soldier
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Combat damage vs. non-combat damage**: Oracle says "combat damage to a player." The trigger uses `TriggerKind::CombatDamageToPlayer` and is only collected when `GameEvent::CombatDamageDealt` fires with a `DamageTarget::Player` target. Non-combat damage fires `GameEvent::NonCombatDamageDealt` and does not collect this trigger. Correct — pass.
- **Source leaves battlefield before trigger resolves**: The engine's `resolve_next_trigger` for `PendingTrigger::CombatDamageToPlayer` does NOT check battlefield presence before dispatching (unlike `CombatDamageToCreature` which does). However, the card's `on_combat_damage_to_player` handler explicitly checks `o.zone == Zone::Battlefield` before calling `add_counters`. If the Patrol has left the battlefield by resolution time, the counter is not placed. Correct per MTG rules — pass.
- **Trigger gating on non-empty description**: The trigger-collection path at `triggers.rs:498-513` only creates the `CombatDamageToPlayer` pending trigger if `trigger_description(...)` returns a non-empty string. The card declares a `TriggeredAbilityDef` with `kind: TriggerKind::CombatDamageToPlayer` and description `"put a +1/+1 counter on Stromkirk Patrol"` (non-empty), so the trigger is always collected — pass.
- **Counter type and amount**: `add_counters(self_id, CounterType::PlusOnePlusOne, 1)` correctly places exactly one +1/+1 counter. `state.add_counters` adds to `objects.get_mut(&id).counters`, which correctly modifies the object regardless of previous counter state — pass.
- **Subtype completeness**: Oracle type line is "Creature — Vampire Soldier." Code has `subtypes: vec!["Vampire".into(), "Soldier".into()]`. Both subtypes present — pass.
- **Mana cost**: Oracle is {4}{B}. Code is `vec![ManaSymbol::Generic(4), ManaSymbol::Colored(Color::Black)]` — pass.
- **P/T**: Oracle 4/3. Code `power: Some(4), toughness: Some(3)` — pass.

### Test coverage
- Basic combat damage → +1/+1 counter: `tier6_cards.rs:455` (`stromkirk_patrol_counter_on_combat_damage`) — TESTED
- Trigger does not fire on non-combat damage to player: NOT TESTED
- Source leaves battlefield between trigger collection and resolution (counter not placed): NOT TESTED
