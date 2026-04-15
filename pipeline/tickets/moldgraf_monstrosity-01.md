---
id: moldgraf_monstrosity-01
status: new
card: Moldgraf Monstrosity
card_file: mtg-engine/src/cards/isd/moldgraf_monstrosity.rs
created: 2026-04-15T03:44:23Z
audit_run_id: 2026-04-14-moldgraf_monstrosity-audit
audit_model: opus
audit_tokens: 15825
audit_duration: 317
---

## Audit Finding

**Oracle text:**
> When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.

**Code:**
> `moldgraf_monstrosity.rs:52`: `state.move_object(object_id, Zone::Exile, registry);` — unconditionally exiles Moldgraf from whatever zone it occupies, with no check that it is still in the graveyard.

**Description:**
The `on_dies` handler unconditionally calls `state.move_object(object_id, Zone::Exile, registry)` without first verifying that Moldgraf is still in `Zone::Graveyard`. Per the first ruling, if Moldgraf has already left the graveyard (e.g., exiled by Tormod's Crypt, or returned to the battlefield by another Moldgraf's trigger), the exile should simply not happen — and the ability should still return two creature cards. Per the second ruling, when two Moldgraf Monstrosities die simultaneously, the first trigger can return the second to the battlefield; the second trigger should then fail to exile it (it's on the battlefield, not in the graveyard) but still return two creatures. The current code would incorrectly exile the second Moldgraf from the battlefield, removing a creature the player just got back.

**Engine path:**
- mtg-engine/src/cards/isd/moldgraf_monstrosity.rs:52

**Required check:** 8j (rulings coverage)

**Affected cards:**
- Moldgraf Monstrosity

## Tests

### moldgraf_exile_skipped_when_not_in_graveyard
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Moldgraf Monstrosity in exile (simulating another effect having moved it). Place two creature cards in the controller's graveyard. Call `on_dies`. Assert that the Moldgraf remains in exile (not double-exiled or moved), and that both graveyard creatures are returned to the battlefield.

### moldgraf_simultaneous_death_second_trigger_does_not_exile_from_battlefield
Source ticket: (new)
Implementation: (not yet written)
Scenario: Create two Moldgraf Monstrosities controlled by the same player. Place two other creature cards in the controller's graveyard. Simulate both Moldgrafs dying simultaneously: move both to graveyard, then resolve triggers in LIFO order. The first trigger exiles Moldgraf A and returns Moldgraf B (now on battlefield) plus one graveyard creature. The second trigger should NOT exile Moldgraf B from the battlefield — assert Moldgraf B remains on the battlefield — and should still return up to two creature cards from the graveyard to the battlefield.

