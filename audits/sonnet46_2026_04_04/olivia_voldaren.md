## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: ISSUE

### Code issues

- `TargetFilter::HasSubtype` in `matches_ability_target_filter` only checks `obj.subtypes`, not registry card data subtypes — ability 1 cannot target real Vampire creature cards
  - `mtg-engine/src/engine.rs` lines 1266–1268 (and duplicated at line 1397 in `matches_target_filter`)
  - Oracle text says: `{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.`
  - Code does: `TargetFilter::HasSubtype(subtype) => { obj.subtypes.contains(subtype) }` — only checks `obj.subtypes`, which is `Vec::new()` for all non-token creature cards. Regular Vampire cards (Markov Patrician, Stromkirk Noble, etc.) store their Vampire subtype in `registry.card_data(card_id).subtypes`, never in `obj.subtypes`. The correct pattern (used by `state.rs::matches_filter` at lines 664–672) checks both. As a result, `generate_ability_targets` (engine.rs line 1309) produces no valid targets for ability 1 when only real Vampire cards are on the battlefield.

- In-card guard check for ability 1 also only checks `obj.subtypes`, missing real Vampire cards
  - `mtg-engine/src/cards/isd/olivia_voldaren.rs` lines 129–131
  - Oracle text says: `{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.`
  - Code does: `let is_vampire = state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield && o.subtypes.contains(&"Vampire".to_string())).unwrap_or(false);` — the same incomplete check. Even if a real Vampire card somehow reached this code path, the guard would prevent the control theft from executing.

### Tricky interactions checked

- **"another" restriction on ability 0**: Enforced at both layers — `TargetFilter::Another` in `matches_ability_target_filter` (engine.rs:1241: `obj.id != source_id`) and a redundant self-exclusion guard in `on_activate_ability` (olivia_voldaren.rs:100: `if *target_id == object_id { return; }`). Pass.
- **Creature becomes Vampire before dying (ruling: 2017-03-14)**: Ability 0 applies the subtype change (`obj.subtypes.push("Vampire".to_string())`) during resolution, before state-based actions run. SBAs are checked after the action completes, so the creature is already a Vampire when lethality is evaluated. Pass.
- **Stolen creatures returned when Olivia leaves the battlefield**: Implemented via `TriggerKind::LeavesBattlefield` → `on_leave_battlefield`. The `LeftBattlefield` trigger is dispatched by `triggers.rs` on `GameEvent::LeftBattlefield`, calls `behavior.on_leave_battlefield` without requiring Olivia to still be on the battlefield (the `LeftBattlefield` resolution arm has no zone check), and `obj.card_state` is preserved across zone changes (only counters, damage, and tap state are cleared in `move_object`). Pass.
- **Ruling: if you lose control of Olivia before ability resolves, ability has no effect**: Activated abilities in this engine resolve immediately when the action is taken (no stack entry for activated abilities). The "before the ability resolves" window from this ruling does not arise. Not applicable.
- **"For as long as you control Olivia" if Olivia changes controllers without leaving the battlefield**: The code reverts stolen creatures only in `on_leave_battlefield`. If Olivia changes controllers but stays on the battlefield, stolen creatures remain under the wrong player indefinitely. However, no card in the current engine's registry has a tested ability to steal Olivia without destroying or exiling her, making this a theoretical gap rather than a demonstrated reachable bug. Not flagged.
- **Ability 0 targeting a creature already a Vampire (from card data)**: Code checks `!obj.subtypes.contains(&"Vampire".to_string())`; since `obj.subtypes` is empty for real cards, it always adds "Vampire" to `obj.subtypes` even for native Vampires. Harmless redundancy: the creature ends up with "Vampire" in `obj.subtypes` in addition to its card-data Vampire status. Pass.
- **LTB trigger dispatch for Olivia**: `triggers.rs` pushes a `LeftBattlefield` trigger for every registered card that leaves the battlefield (no empty-description guard, unlike death-watch). For Olivia, `trigger_description` returns the non-empty description, and `resolve_next_trigger` calls `on_leave_battlefield` correctly. Pass.
- **+1/+1 counter on Olivia for ability 0**: Uses `state.add_counters(object_id, CounterType::PlusOnePlusOne, 1)` which correctly increments the counter map. Pass.

### Test coverage

For each ruling and tricky interaction:
- Ability 0 deals 1 damage and adds Vampire subtype: `mtg-engine/tests/olivia_voldaren.rs:23` (tested)
- Ability 0 retains original subtypes alongside Vampire: `mtg-engine/tests/olivia_voldaren.rs:23` (tested)
- Ability 0 cannot target self ("another"): `mtg-engine/tests/olivia_voldaren.rs:51` (tested)
- Ability 0 puts +1/+1 counter on Olivia: `mtg-engine/tests/olivia_voldaren.rs:23` (tested)
- Ability 1 steals a Vampire: `mtg-engine/tests/olivia_voldaren.rs:68` (tested, but target set up with explicit `obj.subtypes`; does not exercise the bug path of targeting a card whose Vampire status comes from registry)
- Ability 1 rejects non-Vampires: `mtg-engine/tests/olivia_voldaren.rs:86` (tested, but same caveat)
- Ability 1 target filter is `HasSubtype("Vampire")`: `mtg-engine/tests/olivia_voldaren.rs:133` (tested for filter type only, not filtering accuracy against registry-backed Vampires)
- Targeting a real Vampire card (e.g., Markov Patrician placed via `named_creature`) with ability 1: NOT TESTED — this is the scenario that exposes the bug
- Stolen creatures returned when Olivia leaves: `mtg-engine/tests/olivia_voldaren.rs:104` (tested, two stolen creatures)
- Creature becomes Vampire before dying (2017-03-14 ruling): NOT TESTED
- Ruling: control not gained if Olivia lost before resolution: NOT TESTED (also not applicable given engine model)
