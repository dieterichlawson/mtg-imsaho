---
id: splinterfright-02
status: new
card: Splinterfright
card_file: mtg-engine/src/cards/isd/splinterfright.rs
created: 2026-04-14T22:53:48Z
audit_run_id: 2026-04-14-splinterfright-audit
audit_model: opus
audit_tokens: 15601
audit_duration: 5057
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, mill two cards.

**Code:**
> triggers.rs:1306-1308 — `PendingTrigger::UpkeepTrigger { object_id, card_id, chosen_targets, .. } => { if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {`
> splinterfright.rs:53-55 — `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };`

**Description:**
Per CR 112.7a, "Once triggered, an ability exists on the stack independently of its source. Destruction or removal of the source after that time won't affect the ability." If Splinterfright triggers at the beginning of upkeep and is then destroyed before the trigger resolves (e.g., opponent casts a removal spell in response), the mill effect should still resolve — the ability is on the stack independently. However, both the engine-level resolution gate (triggers.rs:1307, `zone == Battlefield` check) and the card's own handler (splinterfright.rs:53-55, same zone check) skip the trigger entirely if the source is no longer on the battlefield. The mill-two-cards effect does not reference the source permanent, so it should resolve fully per CR 603.10 (last-known information is only needed for effects that reference the source).

**Engine path:**
- triggers.rs:1306-1308 (resolution-time zone gate skips trigger if source left battlefield)
- splinterfright.rs:53-55 (card handler also gates on zone==Battlefield)

**Required check:** 8b

**Affected cards:**
- Splinterfright
- All cards with upkeep/end-step/end-combat triggers whose effects don't reference the source permanent (the zone gate at triggers.rs:1300, 1307, 1314 applies to EndCombatTrigger, UpkeepTrigger, and EndStepTrigger identically)

