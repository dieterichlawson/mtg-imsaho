## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Cost floor at {R}: The reduction is capped at `creature_count.min(8)`, keeping the minimum generic portion at 0 so the only remaining symbol is `{R}`. Correct.
- Generic(0) cost symbol when fully reduced: `ManaCost::new(vec![ManaSymbol::Generic(0), ManaSymbol::Colored(Color::Red)])` is produced when ≥8 creatures are present. `try_auto_pay` checks `generic_needed = cost.generic_amount()`, which returns 0 for `Generic(0)`, so only the `{R}` is required. Correct.
- `modified_cost` early-return bypasses global `ReduceCost` continuous effects: `effective_spell_cost` returns immediately when `modified_cost` returns `Some`. However, the only `ReduceCost` effects in the codebase use `SpellFilter::CreatureSpells` or `SpellFilter::CreatureWithSubtype`, neither of which applies to a Sorcery. No cost-increase effects exist in the engine. No practical impact.
- Creature detection via `power.is_some()`: Both `modified_cost` and `on_resolve` use `o.zone == Zone::Battlefield && o.power.is_some()` to identify creatures. This is the established engine-wide convention (non-creature permanents are created with `power: None`); it correctly includes tokens. Consistent with `sba.rs` and the rest of the engine.
- Blast during resolution — Blasphemous Act's own object zone: The spell is popped from the stack before `on_resolve` is called (`stack.rs` line 59), but it remains in `state.objects` with `zone == Zone::Stack` until `move_spell_after_resolve` moves it. The creature filter `zone == Zone::Battlefield` correctly excludes the spell itself (a Sorcery with `power: None`).
- Double `move_spell_after_resolve` call: `on_resolve` calls `move_spell_after_resolve` (moves spell to graveyard), then `resolve_spell` in `stack.rs` checks `if obj.zone == Zone::Stack` before calling it again. Since the zone is already `Graveyard` after the first call, the second call is correctly skipped. No double-move.
- "Each creature" scope — targets both players' creatures: The `on_resolve` filter `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.power.is_some())` includes creatures controlled by all players, matching the oracle's "each creature" (no ownership restriction). Correct.
- Cost locked in before payment: `effective_spell_cost` (which calls `modified_cost`) is invoked at the time the `CastSpell` action is applied (`engine.rs` line 1508), i.e., at announcement time before mana is deducted. This correctly reflects the ruling that the cost is locked in before payment.
- Damage event type: `GameEvent::NonCombatDamageDealt` is used — correct for a spell-based damage effect (not `CombatDamageDealt`).
- `move_spell_after_resolve` used correctly: Blasphemous Act has no flashback cost; `move_spell_after_resolve` sends it to the graveyard. If it were cast with flashback, it would go to exile. Correct.

### Test coverage
- Deals 13 damage to each creature (both players): `tier12_cards.rs:409` — TESTED
- Cost reduction with 5 creatures ({3}{R}): `tier12_cards.rs:434` — TESTED
- Cost reduction with 10 creatures ({R} minimum): `tier12_cards.rs:434` — TESTED
- Castable for {R} with 8 creatures: `tier12_cards.rs:460` — TESTED
- Cost with 0 creatures (None / no modification): `tier12_cards.rs:439` — TESTED
- Cost floor ruling (can't go below {R}): `tier12_cards.rs:450` — TESTED
- Damage applies to opponent's creatures: `tier12_cards.rs:409` — TESTED
- Cost locks in before payment (sacrifice-for-mana scenario): NOT TESTED
- Interaction with cost-increase effects (none in engine): NOT TESTED (no such effects exist)
