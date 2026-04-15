---
id: merged-zone-cleanup-characteristics-02
status: new
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: olivia_voldaren-02, grimoire_of_the_dead-02, creeping_renaissance-01, bitterheart_witch-02, witchbane_orb-02, sturmgeist-01, merged-zone-cleanup-characteristics-01
---

# Zone-change cleanup misses runtime-added characteristics (CR 400.7)

## Description
Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. The engine's `move_object` cleanup block (state.rs:572-583) clears `tapped`, `summoning_sick`, `damage_marked`, `counters`, `is_transformed`, etc. — but does NOT clear runtime-added `subtypes`, `colors`, `card_types`, `attached_to_player`, or `controller`. Cards that mutate these fields at runtime leave stale values that persist into the graveyard, hand, or exile, and survive a round-trip back to the battlefield. Per CR 112.8, a card not on the stack or battlefield is controlled by its owner, so `controller` must also be reset on zone change.

## Engine path
- state.rs:572-583 (move_object cleanup — missing subtypes, colors, card_types, attached_to_player, controller resets)
- state.rs:318-319 (object initialization — these fields start empty)
- state.rs:1564 (attached_to_player field definition)

## Tests

### test_olivia_vampire_subtype_clears_on_zone_change
Source ticket: olivia_voldaren-02
Implementation: (not yet written)
Scenario: Olivia's first ability adds Vampire to a Grizzly Bears. Kill the Bears and reanimate it. Verify the reanimated creature is NOT a Vampire.

### test_grimoire_zombie_subtype_and_color_clear_on_zone_change
Source ticket: grimoire_of_the_dead-02
Implementation: (not yet written)
Scenario: Grimoire of the Dead's sacrifice reanimates a creature, adding Zombie subtype and black color. Kill that creature and return it via another reanimator. Verify the returned creature has neither the Zombie subtype nor black color added.

### test_copied_card_types_clear_on_zone_change
Source ticket: creeping_renaissance-01
Implementation: (not yet written)
Scenario: A creature enters as a copy of an enchantment (gaining Enchantment in card_types). Kill it. Resolve Creeping Renaissance choosing Enchantment. Verify the killed creature is NOT returned (its card_types should revert to printed values in the graveyard).

### test_curse_attached_to_player_clears_on_zone_change
Source ticket: bitterheart_witch-02
Implementation: (not yet written)
Scenario: Bitterheart Witch places a Curse on a player. Bounce the Curse and replay it attached to a different player. Verify attached_to_player reflects the new target and not the stale previous one.

### test_witchbane_orb_stale_attached_to_player_after_zone_change
Source ticket: witchbane_orb-02
Implementation: (not yet written)
Scenario: Put a Curse on the battlefield attached to P0 (set attached_to_player = Some(P0)). Move it to the graveyard via move_object. Assert attached_to_player is None after the zone change. Currently retains Some(P0).

### test_sturmgeist_cda_uses_owner_hand_after_stolen_death
Source ticket: sturmgeist-01
Implementation: (not yet written)
Scenario: Player A owns Sturmgeist. Set Sturmgeist's controller to P1 (simulating a steal). Give P0 3 cards in hand, P1 5 cards. Move Sturmgeist to graveyard. Assert effective_power is 3 (owner P0's hand size), not 5 (thief P1's hand size). Currently fails because controller retains the stale thief value.

## Also closes

- merged-zone-cleanup-characteristics-01

