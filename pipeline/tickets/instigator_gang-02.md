---
id: instigator_gang-02
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
> Attacking creatures you control get +1/+0.
> (back face) Attacking creatures you control get +3/+0.

**Code:**
> `instigator_gang.rs:41-53` — front face declares `TriggerKind::AnyCreatureAttacks` triggered ability. `instigator_gang.rs:93-115` — `on_any_creature_attacks` pushes `TemporaryEffect::ModifyPTWhileSourceInPlay` into `until_end_of_turn`.

**Description:**
"Attacking creatures you control get +1/+0" is a static ability generating a continuous effect (CR 611.3), not a triggered ability. There is no "whenever", "when", or "at" keyword. The implementation models it as an `AnyCreatureAttacks` trigger that fires per-attacker and pushes a temporary P/T modification. While the `ModifyPTWhileSourceInPlay` variant correctly stops applying when the source leaves the battlefield (state.rs:1087), the trigger-based model has structural deficiencies: (a) creatures that enter the battlefield already attacking (e.g., tokens put onto the battlefield tapped and attacking by effects like Hero of Bladehold) do not receive the bonus because no attack trigger fires for them; (b) if Instigator Gang enters the battlefield mid-combat, existing attacking creatures do not retroactively receive the buff; (c) the effect uses the trigger stack rather than applying as a layer-7c continuous effect. This is already tracked as Bug BK in `audits/AUDIT_BUGS.md`.

**Engine path:**
- `instigator_gang.rs:48-53` (AnyCreatureAttacks trigger definition)
- `instigator_gang.rs:93-115` (on_any_creature_attacks handler)
- `triggers.rs:945-966` (attack-watch trigger dispatch)
- `state.rs:1086-1089` (ModifyPTWhileSourceInPlay evaluation)

**Required check:** 8h

**Affected cards:**
- Instigator Gang / Wildblood Pack

