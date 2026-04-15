---
id: merged-zone-cleanup-characteristics-01
status: closed-duplicate
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: olivia_voldaren-02, grimoire_of_the_dead-02, creeping_renaissance-01, bitterheart_witch-02
duplicate_of: merged-zone-cleanup-characteristics-02
---

# Zone-change cleanup misses runtime-added characteristics (CR 400.7)

## Description
Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. The engine's `move_object` cleanup block (state.rs:572-583) clears `tapped`, `summoning_sick`, `damage_marked`, `counters`, `is_transformed`, etc. — but does NOT clear runtime-added `subtypes`, `colors`, `card_types`, or `attached_to_player`. Cards that mutate these fields at runtime (e.g., Olivia turning a creature into a Vampire, Grimoire of the Dead making creatures black Zombies, Creeping Renaissance / copy effects setting card_types, Curse attachments setting attached_to_player) leave stale values that persist into the graveyard, hand, or exile, and — since ObjectId is reused — survive a round-trip back to the battlefield.

## Engine path
- state.rs:572-583 (move_object cleanup — missing subtypes, colors, card_types, attached_to_player resets)
- state.rs:318-319 (object initialization — these fields start empty)
- state.rs:1564 (attached_to_player field definition)
- state.rs:687 (copy effect sets card_types at ETB)
- engine.rs:3760 (copy effect sets card_types mid-game)

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
