## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Grimgrin, Corpse-Born enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Status**: ISSUE

### Code issues

- Auto-sacrifice for the activated ability doesn't present a player choice when multiple sacrifice targets are available.
  - Oracle text says: `Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.`
  - Code does: `engine.rs` lines 1761–1772: `SacrificeCost::SacrificeAnotherCreature` handling calls `.find(|o| o.power.is_some() && o.id != *object_id)` and auto-sacrifices whichever creature comes first in the non-deterministic `HashMap` iteration order, without asking the player to choose. When the controlling player has more than one other creature, the sacrificed creature is chosen arbitrarily rather than by the player. Additionally, `legal_actions` generates only a single `Action::ActivateAbility { targets: vec![] }` action for this ability (engine.rs lines 399–405), providing no mechanism for the player to encode a sacrifice choice.

### Tricky interactions checked

- **Enters tapped**: `on_resolve` explicitly sets `obj.tapped = true` after `move_object(..., Zone::Battlefield)`. `move_object` does not clear `tapped` on entry (only on exit), so the flag is preserved. PASS.
- **Doesn't untap during untap step**: `card_data()` declares `ContinuousEffect::PreventUntap { scope: EffectScope::OnSelf }`. The untap step in `engine.rs` lines 2913–2931 collects locked IDs via `has_continuous_effect`, and those IDs are excluded from the untap batch. `EffectScope::OnSelf` resolves to `creature_id == source_id`, correctly limiting the effect to Grimgrin itself. PASS.
- **Activated ability untap clears tapped without removing the continuous effect**: After the sacrifice ability fires, `obj.tapped = false` is set directly. The `PreventUntap` continuous effect remains on Grimgrin's card data and will correctly prevent it from untapping again on the next untap step. PASS.
- **"Another creature" constraint in sacrifice cost**: Both the legality check (`engine.rs` line 379: `o.id != obj_id`) and the execution (`engine.rs` line 1767: `o.id != *object_id`) correctly exclude Grimgrin itself. PASS.
- **Attack trigger dispatch**: `card_data()` declares `TriggerKind::Attacks`. In `triggers.rs` lines 677–750, a `GameEvent::AttackersDeclared` creates one `PendingTrigger::AttacksTrigger` per attacker whose card has a non-empty `Attacks` trigger description. Grimgrin's description is non-empty. The trigger reaches `on_attacks` via `resolve_next_trigger`. PASS.
- **Defending player lookup**: `on_attacks` uses `state.combat.as_ref().and_then(|c| c.attackers.get(&self_id).copied())` — the `CombatState::attackers` map stores attacker→defending player, so this retrieves the correct defending player for multi-player generality, with a fallback to `state.opponent(controller)`. PASS.
- **"Defending player controls" targeting**: Only creatures (identified by `o.power.is_some()`) belonging to the defending player are included in the target list. The controller's own creatures are excluded. PASS.
- **Counter added even if target is indestructible or regenerates (ruling 2013-07-01)**: `DestroyThenCounter` handler in `engine.rs` lines 2427–2438 calls `try_destroy` then unconditionally calls `add_counters`. No conditional on `try_destroy`'s return value. PASS.
- **No counter if defender has no creatures (ruling 2011-09-22)**: `on_attacks` returns early at line 108–110 if `targets.is_empty()`. No counter is added. PASS.
- **Illegal-target ruling (ruling 2011-09-22, entire ability doesn't resolve)**: In the engine's actual execution flow `process_triggers` resolves attack triggers synchronously — Grimgrin is still on the battlefield when `on_attacks` runs, and target validity was confirmed at collection time. The `DestroyThenCounter` handler in `engine.rs` lines 2427–2438 does not re-check target legality before adding the counter, but this window cannot be reached with an invalid target under the current engine architecture. NOT AN ISSUE in practice.
- **"Then" ordering — destroy before counter**: `engine.rs` lines 2432–2435 call `try_destroy` before `add_counters`. PASS.
- **Summoning sickness not bypassed by sacrifice ability**: `on_activate_ability` sets `tapped = false` but does not modify `summoning_sick`. Grimgrin entering this turn cannot attack even after being untapped. Correct per MTG rules. PASS.
- **is_legendary set correctly**: `on_resolve` explicitly sets `obj.is_legendary = true`, consistent with the pattern used by other legendary cards in the codebase (Geist of Saint Traft, Mikaeus the Lunarch, Grimoire of the Dead). PASS.

### Test coverage

- **Enters tapped**: `grimgrin_enters_tapped` (tier15_cards.rs:1494) — TESTED.
- **Doesn't untap during untap step**: NOT TESTED (no test exercises a full untap step with Grimgrin on the battlefield).
- **Sacrifice untaps Grimgrin and adds +1/+1 counter**: `grimgrin_sacrifice_untaps_and_counters` (tier15_cards.rs:1510) — TESTED (single other creature only; does not test multi-creature choice).
- **Sacrifice ability unavailable without another creature**: `grimgrin_sacrifice_not_available_without_other_creatures` (tier15_cards.rs:1539) — TESTED.
- **Auto-sacrifice with multiple other creatures (player choice)**: NOT TESTED.
- **Attack trigger destroys creature and adds counter (single target, auto-apply)**: `grimgrin_attack_trigger_destroys_and_adds_counter` (tier15_cards.rs:1555) — TESTED.
- **Attack trigger presents choice with multiple targets**: `grimgrin_attack_trigger_presents_choice_with_multiple_targets` (tier15_cards.rs:1581) — TESTED.
- **No counter when defender has no creatures (ruling 2011-09-22)**: `grimgrin_attack_no_targets_no_counter` (tier15_cards.rs:1624) — TESTED.
- **Counter added when target has indestructible (ruling 2013-07-01)**: `grimgrin_attack_indestructible_target_still_gets_counter` (tier15_cards.rs:1647) — TESTED.
- **Attack trigger targets defending player's creatures only**: `grimgrin_attack_uses_defending_player_from_combat` (tier15_cards.rs:1678) — TESTED.
- **Illegal target → no counter (ruling 2011-09-22)**: NOT TESTED.
