---
id: bitterheart_witch-02
status: new
card: Bitterheart Witch
audit_run_id: 2026-04-19-bitterheart_witch-audit
audit_model: sonnet
audit_tokens: 45598
audit_duration: 898
---

## Audit Finding

**Oracle text:**
> put it onto the battlefield attached to target player

**Code:**
> obj.attached_to = None;
// (no corresponding obj.attached_to_player = None in the same cleanup block)

**Description:**
The `move_object` cleanup block (state.rs:599–621) clears `obj.attached_to = None` when any permanent leaves the battlefield, correctly resetting equipment and creature-attached Aura state. However it has no corresponding `obj.attached_to_player = None` for Curses attached to players. Per CR 400.7, zone changes create a new object with no memory of previous existence. A Curse that was placed on the battlefield by Bitterheart Witch (or cast normally via the `resolve_curse` helper) retains `attached_to_player = Some(pid)` in the graveyard, exile, hand, or library after it leaves the battlefield. If any future effect returns the Curse to the battlefield without explicitly setting `attached_to_player` (e.g. an enchantment-reanimation effect), the stale value causes the Curse to enter already attached to a player without legal targeting, bypassing all legality and player-consent requirements. All current consumers of `attached_to_player` happen to check `o.zone == Zone::Battlefield` before reading the field, so no current scenario produces incorrect behavior — but the inconsistency with `attached_to` is an engine invariant violation and will produce silent bugs when enchantment-reanimation is added.

**Engine path:** mtg-engine/src/state.rs:607

**Required check:** 8a

**Affected cards:**
- Curse of Stalked Prey
- Curse of Oblivion
- Curse of the Pierced Heart
- Curse of Death's Hold
- Curse of the Nightly Hunt
- Curse of the Bloody Tome

## Tests

### curse_attached_to_player_cleared_on_zone_change
Scenario: Witch's trigger attaches Curse of Death's Hold to opponent; Naturalize destroys the Curse; graveyard object's attached_to_player field should be None, not the opponent's PlayerId

