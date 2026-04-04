## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
**Type line**: Creature — Elemental Dog
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Damage type (NonCombatDamageDealt vs CombatDamageDealt)**: `deal_1_damage` pushes `GameEvent::NonCombatDamageDealt`. This is correct — the oracle says "deals 1 damage" from a triggered ability, not combat damage. Pass.
- **Blocks trigger dispatch**: `triggers.rs` lines 752–773 dispatch `BlocksTrigger` for the blocker when `BlockersDeclared` fires, with `blocked_attacker` correctly set to the attacking creature. `resolve_next_trigger` (lines 987–993) calls `behavior.on_blocks(state, hound_id, blocked_attacker)`. The `on_blocks` handler deals 1 damage to `blocked_attacker` (the creature Ashmouth Hound is blocking). Pass.
- **BecomesBlocked trigger dispatch**: `triggers.rs` lines 800–820 dispatch `BecomesBlockedTrigger` for the attacker (Ashmouth Hound) when a blocker is assigned to it, with `blocker_id` set to the blocking creature. `resolve_next_trigger` (lines 1001–1007) calls `behavior.on_becomes_blocked(state, hound_id, blocker_id)`. The `on_becomes_blocked` handler deals 1 damage to `blocker_id`. Pass.
- **One trigger per creature (ruling)**: The `BlockersDeclared` loop in `triggers.rs` iterates over each `(blocker_id, attacker_id)` pair independently. If Ashmouth Hound is attacked and blocked by two creatures, two separate `BecomesBlockedTrigger` entries are created, resulting in 1 damage dealt to each blocker. This matches the ruling: "triggers once for each creature it blocks or becomes blocked by." Pass.
- **Trigger description non-empty gate**: Both `TriggeredAbilityDef` entries have non-empty descriptions (`"deal 1 damage to blocked creature"` and `"deal 1 damage to blocking creature"`). The dispatch code only creates triggers when `!desc.is_empty()`. Both triggers will fire. Pass.
- **Zone check before dealing damage**: `deal_1_damage` checks `obj.zone == Zone::Battlefield` before applying damage, preventing damage to a creature that has already left. Pass.
- **Subtype data correctness**: `subtypes: vec!["Elemental".into(), "Dog".into()]` matches the type line "Creature — Elemental Dog". (The inline comment on line 6 says "Elemental Hound" but this is a documentation-only error that has no effect on behavior.) Pass.
- **Card leaves battlefield between trigger and resolution**: The `BlocksTrigger` and `BecomesBlockedTrigger` resolution paths both check `state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before calling the handler. If Ashmouth Hound leaves the battlefield before its trigger resolves, the damage ability correctly does not fire (appropriate for an ability that references "this creature"). Pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Ashmouth Hound deals 1 damage when it blocks (Blocks trigger): `mtg-engine/tests/tier12_cards.rs:169` (`ashmouth_hound_deals_damage_on_block`)
- Ashmouth Hound deals 1 damage when it becomes blocked (BecomesBlocked trigger, Hound as attacker): NOT TESTED
- One trigger per blocking creature when multiple creatures block Ashmouth Hound: NOT TESTED
- Trigger does not fire if Ashmouth Hound leaves battlefield before resolution: NOT TESTED
