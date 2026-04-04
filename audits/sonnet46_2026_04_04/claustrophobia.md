## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
When this Aura enters, tap enchanted creature.
Enchanted creature doesn't untap during its controller's untap step.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- ETB trigger fires correctly through trigger stack (not inline): PASS — `collect_triggers` creates `PendingTrigger::EnteredBattlefield` from the `GameEvent::EnteredBattlefield` event; `resolve_next_trigger` calls `on_enter_battlefield` only if the aura is still on the battlefield (correct: if the aura has left, `attached_to` is already cleared to `None`, so no creature would be tapped regardless).
- Targeting a creature that is already tapped: PASS — `TargetRequirement::Creature` places no restriction on tapped state, matching the ruling "Claustrophobia can target and enchant a tapped or untapped creature." The ETB tap sets `tapped = true` on an already-tapped creature (no-op, harmless).
- "Its controller's untap step" scoping: PASS — `engine.rs` untap logic calls `objects_in_zone(Zone::Battlefield, active)` to build the locked list. Only the active player's permanents are considered for untapping, so the enchanted creature (controlled by its owner) is correctly locked only during that player's untap step.
- PreventUntap is continuously evaluated, not snapshot at ETB: PASS — `ContinuousEffect::PreventUntap { scope: EffectScope::Attached }` is evaluated freshly each untap step via `has_continuous_effect`. There is no snapshot taken at ETB time.
- Aura SBA when enchanted creature leaves battlefield: PASS — `sba.rs` rule 704.5m correctly moves Claustrophobia to graveyard when `attached_to` target is no longer on the battlefield.
- Creature can be untapped by other means, Claustrophobia remains attached: PASS — nothing in the code detaches Claustrophobia or removes the continuous effect when the creature is untapped by a non-untap-step source.
- Claustrophobia caster ≠ enchanted creature's controller (aura across controllers): PASS — `has_continuous_effect` scans `state.objects.values()` (all battlefield objects regardless of controller). `effect_applies_to` with `EffectScope::Attached` checks only `source.attached_to == Some(creature_id)`, which is controller-agnostic.
- `attached_to` is set before the ETB trigger is collected: PASS — `resolve_aura` calls `state.move_object(aura_id, Zone::Battlefield)` (which enqueues the `EnteredBattlefield` event) and then immediately sets `obj.attached_to = Some(*target_id)`. Because event collection happens after resolution, `attached_to` is already set by the time `collect_triggers` processes the event.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- ETB tap on untapped creature: `mtg-engine/tests/innistrad_cards.rs:323` (`claustrophobia_taps_creature`)
- Prevent untap during controller's untap step: `mtg-engine/tests/card_mechanics.rs:490` (`claustrophobia_prevents_untap`)
- Normal creature still untaps alongside Claustrophobia-enchanted creature: `mtg-engine/tests/card_mechanics.rs:490` (same test asserts the normal creature untaps)
- Targeting an already-tapped creature: NOT TESTED
- Creature untapped by other means (e.g., Burst of Speed), Claustrophobia remains attached: NOT TESTED
- SBA removes Claustrophobia when enchanted creature leaves battlefield: NOT TESTED
- Claustrophobia bounce before ETB trigger resolves (trigger countered, no tap): NOT TESTED
