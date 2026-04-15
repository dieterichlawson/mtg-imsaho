---
id: witchbane_orb-02
status: closed-duplicate
card: Witchbane Orb
card_file: mtg-engine/src/cards/isd/witchbane_orb.rs
created: 2026-04-15T03:51:04Z
audit_run_id: 2026-04-14-witchbane_orb-audit
audit_model: opus
audit_tokens: 10258
audit_duration: 267
duplicate_of: merged-zone-cleanup-characteristics-02
---

## Audit Finding

**Oracle text:**
> When this artifact enters, destroy all Curses attached to you.

**Code:**
> Zone-change cleanup block in `move_object` (state.rs:572-583) clears `attached_to = None` (line 577) but does NOT clear `attached_to_player`.

**Description:**
When a curse leaves the battlefield (e.g., destroyed, bounced, exiled), `move_object` clears `attached_to` but not `attached_to_player`. Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. If a curse is destroyed and later returned to the battlefield by a non-cast effect (e.g., Replenish, Open the Vaults), it enters without going through `resolve_curse()` — so no new `attached_to_player` is set. But the stale value from the previous zone persists. Witchbane Orb's ETB checks `o.attached_to_player == Some(controller)`, so it would incorrectly detect the re-entered curse as "attached to you" and destroy it, even though the curse is not properly attached to any player. Conversely, if the curse was previously attached to the opponent and is re-entered under the Orb controller's side, Witchbane Orb would miss it (stale value points to the wrong player).

**Engine path:**
- state.rs:572-583 (zone-change cleanup — missing `attached_to_player` reset)
- state.rs:577 (`attached_to = None` — the analogous field IS cleared)
- witchbane_orb.rs:50 (reads `o.attached_to_player`)

**Required check:** 8a

**Affected cards:**
- Witchbane Orb (false positive/negative curse detection)
- All curse cards (stale attachment after zone change)
- Any card that reads `attached_to_player` (e.g., curses with `ControlledByAttachedPlayer` effects per state.rs:883-886)

## Tests

### stale_attached_to_player_after_zone_change
Source ticket: (new)
Implementation: (not yet written)
Scenario: Put a Curse on the battlefield attached to P0 (set `attached_to_player = Some(P0)`). Move it to the graveyard via `move_object`. Assert `attached_to_player` is `None` after the zone change. Currently it will retain `Some(P0)`.
