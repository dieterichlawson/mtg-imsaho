---
id: geist_of_saint_traft-03
status: new
card: Geist of Saint Traft
card_file: mtg-engine/src/cards/isd/geist_of_saint_traft.rs
created: 2026-04-14T21:28:51Z
audit_run_id: 2026-04-14-geist_of_saint_traft-audit
audit_model: opus
audit_tokens: 20037
audit_duration: 493
---

## Audit Finding

**Oracle text:**
> Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking.

**Code:**
> `let defender = state.opponent(controller);` (geist_of_saint_traft.rs:71)
> `combat.attackers.insert(token_id, defender);` (geist_of_saint_traft.rs:81)
> `pub attackers: HashMap<ObjectId, PlayerId>,` (state.rs:1698)

**Description:**
The code hardcodes the Angel's defender as `state.opponent(controller)` — the single opponent in a 2-player game. Per the ruling, the controller should choose which player or planeswalker the Angel attacks, and it need not match Geist's target. The engine's `CombatState.attackers` maps attacker IDs to `PlayerId` only (state.rs:1698), with no support for planeswalkers as attack targets. `DeclareAttackers` (actions.rs:66) also only accepts `PlayerId`. If the opponent controls planeswalkers, the Angel token cannot be directed at them. This is an engine-wide limitation — the combat system does not support attacking planeswalkers for any card — but it directly causes Geist to violate this ruling.

**Engine path:**
- geist_of_saint_traft.rs:71 — hardcodes defender to opponent
- state.rs:1698 — CombatState.attackers typed as HashMap<ObjectId, PlayerId>
- actions.rs:66 — DeclareAttackers only accepts PlayerId

**Required check:** 8j

**Affected cards:**
- Geist of Saint Traft
- All cards that create tokens attacking (Kessig Cagebreakers, Hero of Bladehold, etc.)
- All attacking creatures (engine-wide: no planeswalker attack support)

