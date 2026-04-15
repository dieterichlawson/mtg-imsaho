---
id: geist_of_saint_traft-02
status: deduped
card: Geist of Saint Traft
card_file: mtg-engine/src/cards/isd/geist_of_saint_traft.rs
created: 2026-04-14T21:28:51Z
audit_run_id: 2026-04-14-geist_of_saint_traft-audit
audit_model: opus
audit_tokens: 20037
audit_duration: 493
deduped_into: merged-trigger-source-zone-gate-01
---

## Audit Finding

**Oracle text:**
> Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking.

**Code:**
> `PendingTrigger::AttacksTrigger { object_id, card_id, chosen_targets, .. } => { if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {` (triggers.rs:1332-1333)
> `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };` (geist_of_saint_traft.rs:54-57)

**Description:**
The attack trigger resolution in `resolve_next_trigger` gates `AttacksTrigger` dispatch on the source object being on the battlefield (triggers.rs:1332-1333). The card's own `on_attacks` handler also early-returns if Geist is not on the battlefield (line 54-57). Per CR 603.3, once a triggered ability triggers, it goes on the stack independently of its source. Per CR 113.7a, if the source has changed zones, the ability uses last-known information. The effect — creating a token — does not reference the source object and should resolve regardless of whether Geist is still on the battlefield. If an opponent responds to the attack trigger by destroying Geist (e.g., with an instant that bypasses hexproof, or after hexproof is removed), the Angel token is never created. The battlefield gate is an engine-wide pattern affecting all `AttacksTrigger` resolution, not just Geist.

**Engine path:**
- triggers.rs:1332-1333 — AttacksTrigger resolution battlefield gate
- geist_of_saint_traft.rs:54-57 — on_attacks handler battlefield gate

**Required check:** 8b

**Affected cards:**
- Geist of Saint Traft
- All cards with `TriggerKind::Attacks` whose effects don't reference the source (e.g., Kessig Cagebreakers, Hamlet Captain)
