---
id: instigator_gang-01
status: new
card: Instigator Gang
card_file: mtg-engine/src/cards/isd/instigator_gang.rs
created: 2026-04-14T21:28:14Z
audit_run_id: 2026-04-14-instigator_gang-audit
audit_model: opus
audit_tokens: 13518
audit_duration: 264
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if a player cast two or more spells last turn, transform Wildblood Pack.

**Code:**
> `instigator_gang.rs:71-78` — `back_face_data()` returns `triggered_abilities` containing only `AnyCreatureAttacks`; no `TriggerKind::Upkeep` entry exists.

**Description:**
The back face (Wildblood Pack) is missing the Upkeep trigger definition in its `triggered_abilities` vec. The upkeep trigger dispatch in `triggers.rs:832` uses `face_trigger_description` (triggers.rs:492), which only checks the currently visible face's triggered abilities. When the card is transformed (Wildblood Pack), `is_transformed` is true, so the dispatch looks at `back_face_data().triggered_abilities` — which has no Upkeep entry. The description comes back empty and no trigger is created. This means Wildblood Pack can never transform back to Instigator Gang via the upkeep trigger. Other werewolves (e.g., Daybreak Ranger at `daybreak_ranger.rs:67-72`) correctly include `TriggerKind::Upkeep` in their back face data.

**Engine path:**
- `instigator_gang.rs:71` (back_face_data triggered_abilities)
- `triggers.rs:832` (upkeep trigger dispatch)
- `triggers.rs:492` (face_trigger_description only checks current face)

**Required check:** 8b

**Affected cards:**
- Instigator Gang / Wildblood Pack

