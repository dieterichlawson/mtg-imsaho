---
id: snapcaster_mage-02
status: deduped
card: Snapcaster Mage
card_file: mtg-engine/src/cards/isd/snapcaster_mage.rs
created: 2026-04-14T20:56:41Z
audit_run_id: 2026-04-14-snapcaster_mage-audit
audit_model: opus
audit_tokens: 19116
audit_duration: 381
deduped_into: merged-trigger-target-recheck-01
---

## Audit Finding

**Oracle text:**
> When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn.

**Code:**
> `triggers.rs:1242-1249`: The `EnteredBattlefield` match arm calls `behavior.on_enter_battlefield(state, object_id, &chosen_targets, registry)` without any target legality re-check.
> `snapcaster_mage.rs:61`: `let Some(Target::Object(target_id)) = chosen_targets.first() else { return };` — checks that the object exists, but not that it is still in the graveyard.

**Description:**
Per CR 608.2b, when a triggered ability with targets resolves, the game checks whether each target is still legal. For Snapcaster's trigger, the target must still be an instant or sorcery card in the controller's graveyard. The engine's `resolve_next_trigger` at triggers.rs:1242 does not perform any target legality check before dispatching to `on_enter_battlefield`. If an opponent exiles the targeted card in response to the trigger (e.g., with Purify the Grave), the trigger should fizzle. Instead, it resolves and grants flashback to a card that is now in exile. While the flashback cannot actually be used from exile (the offering code only checks graveyard), the trigger incorrectly resolves rather than being countered by game rules.

**Engine path:**
- triggers.rs:1242-1249 (trigger resolution — no target check)
- snapcaster_mage.rs:58-73 (`on_enter_battlefield` — no zone check on target)

**Required check:** 8b (trigger dispatch) + step 6 ("target" rules)

**Affected cards:**
- Snapcaster Mage
- Every card with a targeted triggered ability that resolves through `resolve_next_trigger` (Slayer of the Wicked, Morkrut Banshee, Fiend Hunter, etc.)
