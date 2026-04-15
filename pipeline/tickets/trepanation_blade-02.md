---
id: trepanation_blade-02
status: new
card: Trepanation Blade
card_file: mtg-engine/src/cards/isd/trepanation_blade.rs
created: 2026-04-15T03:52:30Z
audit_run_id: 2026-04-14-trepanation_blade-audit
audit_model: opus
audit_tokens: 28121
audit_duration: 580
---

## Audit Finding

**Oracle text:**
> Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.

**Code:**
> `trepanation_blade.rs:59-65`:
> ```rust
> let Some(equip) = state.get_object(self_id) else { return; };
> let Some(creature_id) = equip.attached_to else { return; };
>
> let defending_player = state.combat.as_ref()
>     .and_then(|c| c.attackers.get(&creature_id).copied());
> let Some(defending_player) = defending_player else { return; };
> ```

**Description:**
The `on_attacks` handler derives the attacking creature and defending player from the equipment's _current_ state at resolution time (`equip.attached_to` → creature → `combat.attackers` → defender). Per CR 603.3c and 608.2g, a triggered ability uses last-known information about the triggering event. The trigger fires when the equipped creature attacks — at that moment, the creature's identity and the defending player are known. If the equipped creature is destroyed in response (e.g., opponent blocks and uses a first-strike trick, or uses instant removal), SBA at sba.rs:147-165 detaches the equipment (`attached_to = None`). The handler then fails at line 60 and returns early, performing _none_ of the ability's effects — not even the mill. Per CR 608.2b, the ability should resolve doing as much as possible: the defending player's library should still be milled, with only the +1/+0 buff failing (since the creature is gone). The root cause is that `PendingTrigger::AttacksTrigger` (triggers.rs:132-137) has no field for the attacking creature ID or defending player; for equipment triggers, this information must be captured at trigger-creation time.

**Engine path:**
- trepanation_blade.rs:59-65 (reads current state)
- triggers.rs:132-137 (AttacksTrigger struct lacks creature_id/defending_player fields)
- triggers.rs:921-944 (trigger creation for equipment — knows creature_id but doesn't store it)
- sba.rs:147-165 (equipment detachment clears attached_to)

**Required check:** 8b

**Affected cards:**
- Trepanation Blade
- Any equipment with attack-triggered abilities (all equipment triggers derive creature from `attached_to`)

## Tests

### attack_trigger_resolves_after_creature_destroyed
Source ticket: (new)
Implementation: (not yet written)
Scenario: Equip Trepanation Blade to a creature. Set up defender's library with [nonland, nonland, land]. Declare the creature as an attacker, putting the Blade's trigger on the stack. Before the trigger resolves, destroy the equipped creature (move to graveyard). Run SBAs so equipment detaches. Resolve the trigger. Assert that the defender's library was milled (all 3 cards in graveyard). The +3/+0 buff should NOT apply (creature is gone), but the mill must still occur.

