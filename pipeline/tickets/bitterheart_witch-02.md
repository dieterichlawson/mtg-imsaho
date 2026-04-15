---
id: bitterheart_witch-02
status: closed-duplicate
card: Bitterheart Witch
card_file: mtg-engine/src/cards/isd/bitterheart_witch.rs
created: 2026-04-14T21:20:46Z
audit_run_id: 2026-04-14-bitterheart_witch-audit
audit_model: opus
audit_tokens: 13463
audit_duration: 293
duplicate_of: merged-zone-cleanup-characteristics-01
---

## Audit Finding

**Oracle text:**
> (General rule) CR 400.7: An object that moves from one zone to another becomes a new object with no memory of its previous existence.

**Code:**
> Zone-change cleanup block in `move_object` (state.rs:572-583) clears `attached_to` but does NOT clear `attached_to_player`:
> ```rust
> obj.attached_to = None;
> // attached_to_player is NOT cleared
> ```

**Description:**
When the `move_object` function processes a permanent leaving the battlefield (state.rs:572-583), it resets `attached_to` to `None` but does not reset `attached_to_player`. Per CR 400.7, an object that changes zones becomes a new object with no memory of, among other things, what it was attached to. If a Curse placed by Bitterheart Witch later leaves the battlefield (e.g., via Disperse bouncing it, or exile-and-return effects) and then re-enters, the stale `attached_to_player` value persists. This could cause the Curse to appear still attached to its old target, or interact incorrectly with effects that check `attached_to_player`. This affects all Curse auras, not just those placed by Bitterheart Witch.

**Engine path:**
- state.rs:572-583 (zone-change cleanup block — `attached_to_player` missing from reset)
- state.rs:1564 (field definition: `pub attached_to_player: Option<PlayerId>`)

**Required check:** 8a

**Affected cards:**
- Bitterheart Witch (places Curses via this path)
- Curse of the Pierced Heart
- Curse of Death's Hold
- Curse of Stalked Prey
- Curse of the Bloody Tome
- Curse of Oblivion
- Curse of the Nightly Hunt
- Any card with `attached_to_player`
