---
id: kessig_cagebreakers-01
status: closed-duplicate
card: Kessig Cagebreakers
card_file: mtg-engine/src/cards/isd/kessig_cagebreakers.rs
created: 2026-04-14T21:13:42Z
audit_run_id: 2026-04-14-kessig_cagebreakers-audit
audit_model: opus
audit_tokens: 17473
audit_duration: 419
duplicate_of: merged-trigger-source-zone-gate-01
---

## Audit Finding

**Oracle text:**
> Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.

**Code:**
> `triggers.rs:1333`: `if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield)`
> `kessig_cagebreakers.rs:41-44`: `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };`

**Description:**
Both the engine's attack-trigger resolution path and the card's own `on_attacks` handler bail out if the source creature is no longer on the battlefield. Per CR 603.3, a triggered ability on the stack exists independently of its source — destruction or removal of the source after the trigger is placed on the stack does not affect the triggered ability. If Kessig Cagebreakers is removed from the battlefield in response to its attack trigger (e.g., opponent casts instant-speed removal), the trigger should still resolve and create Wolf tokens. The effect text does not reference the source object — it only says "create ... tokens ... for each creature card in your graveyard" — so there is no need for the source to be present. Furthermore, if Cagebreakers itself dies before the trigger resolves, it becomes a creature card in the controller's graveyard and should be counted toward the total (per ruling: "You count the number of creature cards in your graveyard when the triggered ability resolves"), producing one additional Wolf token. Currently, zero tokens are created in this scenario.

**Engine path:**
- triggers.rs:1332-1337 (resolution dispatch — blanket zone check on all AttacksTrigger variants)
- kessig_cagebreakers.rs:41-44 (card-level zone check)

**Required check:** 8b

**Affected cards:**
- Kessig Cagebreakers
- Geist of Saint Traft (same pattern at geist_of_saint_traft.rs:54-57)
- Grimgrin, Corpse-Born (same pattern at grimgrin_corpse_born.rs — though its effect references the source for +1/+1 counter, the "destroy target creature" portion should still resolve)
- Hamlet Captain (same pattern at hamlet_captain.rs:45-47)
- Trepanation Blade (same pattern at trepanation_blade.rs:57-60)
- All cards using `TriggerKind::Attacks` with `on_attacks` handlers
