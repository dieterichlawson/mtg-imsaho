---
id: merged-inline-damage-01
status: closed-duplicate
card: multiple
created: 2026-04-15T02:45:28Z
kind: consolidated
source_tickets: balefire_dragon-01, olivia_voldaren-01, daybreak_ranger-01, devils_play-01, into_the_maw_of_hell-01
duplicate_of: merged-inline-damage-02
---

# Inline damage writes bypass the central damage handler (CR 702.16e, 614.1a)

## Description
Several cards apply damage by mutating `obj.damage_marked += amount` directly instead of routing through `apply_pending_effect(PendingEffect::DealDamage)` at engine.rs:3424. The central handler enforces (a) damage prevention / replacement effects (engine.rs:3426-3447, including Unbreathing Horde's counter-remove-to-prevent), (b) protection from source via `has_protection_from` (engine.rs:3449-3453, CR 702.16e), and (c) planeswalker loyalty-counter removal for planeswalker targets (engine.rs:3460-3466, CR 120.3c). Inline writes skip all three, so protection, prevention, and planeswalker targeting are silently incorrect.

## Engine path
- engine.rs:3424-3478 (central PendingEffect::DealDamage handler — what all sources should use)
- engine.rs:3449-3453 (protection-from-source check bypassed by inlining)
- engine.rs:3426-3447 (PreventDamageRemoveCounter and other replacement effects bypassed)
- engine.rs:3460-3466 (planeswalker loyalty removal bypassed)
- helpers.rs:49-83 (`resolve_damage` helper that also inlines damage)

## Tests

### test_balefire_dragon_respects_protection_from_red
Source ticket: balefire_dragon-01
Implementation: (not yet written)
Scenario: Balefire Dragon deals combat damage to a player who controls a creature with protection from red. Verify that creature takes 0 damage from the triggered ability.

### test_olivia_voldaren_first_ability_respects_protection
Source ticket: olivia_voldaren-01
Implementation: (not yet written)
Scenario: Activate Olivia's {1}{R} ability targeting a creature with protection from black. Verify no damage is dealt, no Vampire subtype added, and no +1/+1 counter placed on Olivia.

### test_daybreak_ranger_ability_respects_protection
Source ticket: daybreak_ranger-01
Implementation: (not yet written)
Scenario: Activate Daybreak Ranger's {T} ability targeting a flying creature with protection from green. Verify no damage is dealt.

### test_devils_play_removes_planeswalker_loyalty
Source ticket: devils_play-01
Implementation: (not yet written)
Scenario: Cast Devil's Play for X=3 targeting an opponent's planeswalker with 4 loyalty. Verify loyalty drops to 1 (removed by counter-removal) and the planeswalker's `damage_marked` is 0.

### test_into_the_maw_of_hell_respects_protection
Source ticket: into_the_maw_of_hell-01
Implementation: (not yet written)
Scenario: Cast Into the Maw of Hell targeting a land and a creature with protection from red. Verify the creature takes 0 damage.
