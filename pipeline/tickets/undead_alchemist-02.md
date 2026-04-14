---
id: undead_alchemist-02
status: new
card: Undead Alchemist
card_file: mtg-engine/src/cards/isd/undead_alchemist.rs
created: 2026-04-14T21:13:20Z
audit_run_id: 2026-04-14-undead_alchemist-audit
audit_model: opus
audit_tokens: 16895
audit_duration: 397
---

## Audit Finding

**Oracle text:**
> Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.

**Code:**
> `undead_alchemist.rs:61`: `state.move_object(milled_object, Zone::Exile, registry);` — called unconditionally without checking the milled card's current zone

**Description:**
The `on_creature_card_milled` handler exiles the milled creature card without verifying it is still in the graveyard. Per CR 608.2d, if an instruction tells a player to perform an impossible action (exiling a card that is no longer in the expected zone), that part is simply not performed, but the rest of the instruction resolves normally. In the common case of multiple Undead Alchemists (ruling 2), the first trigger exiles the card and the second trigger calls `move_object` on a card already in exile — this is functionally benign (exile-to-exile just bumps `zone_change_count`), and the token is correctly created regardless. However, if between the two triggers resolving an opponent moves the card from exile to another zone (e.g., hand via an instant-speed effect), the second trigger would incorrectly exile it from that zone. The handler should check `state.get_object(milled_object).is_some_and(|o| o.zone == Zone::Graveyard)` before the `move_object` call, and still create the token regardless of the check result.

**Engine path:**
- undead_alchemist.rs:61

**Required check:** 8j (ruling 2 verification)

**Affected cards:**
- Undead Alchemist

