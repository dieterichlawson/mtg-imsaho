---
id: merged-creature-type-proxy-01
status: new
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: geist_honored_monk-01, moorland_haunt-01, spider_spawning-01
---

# `power.is_some()` used as creature-card proxy (CR 302.1)

## Description
Per CR 302.1, a creature card is defined by having the Creature card type, not by having power/toughness. Several cards and engine helpers use `o.power.is_some()` as a proxy for "is a creature card." This is semantically incorrect: it would include non-creature cards with printed P/T (e.g., Vehicles are Artifact cards with P/T but not Creature types until crewed) and could exclude theoretical creatures without P/T. The correct check is `o.card_types.contains(&CardType::Creature)`. Some cards (Grimoire of the Dead, Graveyard Shovel) already use the correct dual check; others fell back to the proxy. Not functionally visible in current Innistrad-only card pool but inconsistent with CR.

## Engine path
- Engine-wide convention using `o.power.is_some()` — see geist_honored_monk.rs:42, moorland_haunt.rs:53, spider_spawning.rs:37
- Correct pattern: `o.card_types.contains(&CardType::Creature)`
- state.rs:1414 (check_condition — uses the correct dual subtype lookup)

## Tests

### test_geist_honored_monk_dynamic_pt_uses_card_types
Source ticket: geist_honored_monk-01
Implementation: (not yet written)
Scenario: Ensure Geist-Honored Monk's dynamic P/T counts creatures correctly under a synthetic scenario where a non-creature artifact has printed P/T (or document the engine-wide pattern and assert a unit-level switch to `card_types.contains(Creature)`).

### test_moorland_haunt_exile_filter_uses_card_types
Source ticket: moorland_haunt-01
Implementation: (not yet written)
Scenario: Have a Vehicle artifact card with P/T in the graveyard. Verify Moorland Haunt's activation does not offer that Vehicle as a valid exile target (only true creature cards).

### test_spider_spawning_counts_card_types
Source ticket: spider_spawning-01
Implementation: (not yet written)
Scenario: Have a Vehicle artifact card with P/T in the graveyard plus N true creature cards. Cast Spider Spawning. Verify N Spider tokens are created, not N+1.

