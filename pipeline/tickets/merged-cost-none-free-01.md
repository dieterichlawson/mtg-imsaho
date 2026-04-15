---
id: merged-cost-none-free-01
status: new
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: past_in_flames-01, snapcaster_mage-01
---

# Cards with `cost: None` get free flashback via `ManaCost::free()` fallback

## Description
When a grant-flashback source targets a card whose `CardData.cost` is `None` (no mana cost — suspend-only cards like Ancestral Vision or Living End), both Snapcaster Mage's handler and the engine's `PendingEffect::GrantFlashback` resolver call `unwrap_or_else(ManaCost::free)`. This produces an empty mana cost (`{0}`), making the card castable via flashback for free. Per CR 107.2 and the Scryfall rulings for Snapcaster Mage and Past in Flames, a card with no mana cost cannot be cast via granted flashback — the grant should either be skipped or record `cost: None` so the flashback-offering path rejects it.

## Engine path
- snapcaster_mage.rs:63-65 (on_enter_battlefield — unwrap_or_else(ManaCost::free))
- past_in_flames.rs:53 (unwrap_or(ManaCost::free()))
- engine.rs:3922-3924 (PendingEffect::GrantFlashback resolver — same pattern)
- engine.rs:1226-1228 (offering path reads GrantFlashback.cost and offers the spell)
- engine.rs:2221-2224 (casting path uses stored cost directly)

## Tests

### test_past_in_flames_skips_no_cost_cards
Source ticket: past_in_flames-01
Implementation: (not yet written)
Scenario: Have a hypothetical no-mana-cost instant (cost: None) in graveyard. Resolve Past in Flames. Verify that instant is NOT offered as castable via flashback (not just "free").

### test_snapcaster_cannot_target_no_cost_cards
Source ticket: snapcaster_mage-01
Implementation: (not yet written)
Scenario: Have a hypothetical no-mana-cost instant (cost: None) in graveyard. ETB Snapcaster. Verify that instant is not a legal target (or the resulting flashback grant does not make the card castable for free).

