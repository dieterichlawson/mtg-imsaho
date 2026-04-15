---
id: undead_alchemist-01
status: closed-duplicate
card: Undead Alchemist
card_file: mtg-engine/src/cards/isd/undead_alchemist.rs
created: 2026-04-14T21:13:20Z
audit_run_id: 2026-04-14-undead_alchemist-audit
audit_model: opus
audit_tokens: 16895
audit_duration: 397
duplicate_of: merged-trigger-source-zone-gate-02
---

## Audit Finding

**Oracle text:**
> Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.

**Code:**
> `triggers.rs:1374`: `if state.get_object(watcher_id).is_some_and(|o| o.zone == Zone::Battlefield)`
> `undead_alchemist.rs:56-59`: `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };`

**Description:**
The trigger resolution path for `CreatureCardMilledWatch` checks that the Undead Alchemist is still on the battlefield before dispatching the handler. Per CR 113.7a, "Once activated or triggered, an ability exists on the stack independently of its source. Destruction or removal of the source after that time won't affect the ability." If the Alchemist leaves the battlefield after the trigger is placed on the stack but before it resolves (e.g., the Alchemist takes lethal combat damage in the same combat step that triggers the mill), the trigger silently does nothing — no exile, no token. The controller is already stored in the `PendingTrigger::CreatureCardMilledWatch` struct (field `controller` at triggers.rs:1090), but it is discarded at resolution time in favor of re-reading the object (which may no longer exist). The handler at `on_creature_card_milled` has a redundant battlefield gate that compounds the problem. This is an engine-wide pattern: all watcher-type trigger resolutions (SpellCastWatch, AttackWatch, CreatureCardMilledWatch) have the same gate at resolution time (triggers.rs:1320-1351).

**Engine path:**
- triggers.rs:1373-1379
- undead_alchemist.rs:56-59

**Required check:** 8b

**Affected cards:**
- Undead Alchemist
- Any card using watcher-type triggers (SpellCastWatch, AttackWatch, CreatureCardMilledWatch, etc.) — engine-wide pattern
