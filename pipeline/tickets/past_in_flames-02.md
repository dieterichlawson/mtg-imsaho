---
id: past_in_flames-02
status: new
card: Past in Flames
card_file: mtg-engine/src/cards/isd/past_in_flames.rs
created: 2026-04-14T20:57:22Z
audit_run_id: 2026-04-14-past_in_flames-audit
audit_model: opus
audit_tokens: 20370
audit_duration: 422
---

## Audit Finding

**Oracle text:**
> Each instant and sorcery card in your graveyard gains flashback until end of turn.

**Code:**
> `TemporaryEffect::GrantFlashback { target: ObjectId, cost: ManaCost }` — `state.rs:209`
> Engine flashback lookup: `if *target == obj.id { Some(cost.clone()) }` — `engine.rs:1228`

**Description:**
The `GrantFlashback` temporary effect targets cards by `ObjectId`, but the engine reuses `ObjectId` through zone changes (incrementing `zone_change_count` instead of creating new IDs). If a card that received PiF-granted flashback leaves the graveyard (e.g., exiled by Purify the Grave, returned to hand by Make a Wish) and then returns to the graveyard later in the same turn, the stale `GrantFlashback` entry still matches the card's ObjectId. Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. The returned card is a new object and should NOT have flashback from PiF's earlier resolution. The `GrantFlashback` entry should either store a `zone_change_count` and validate it on lookup, or be invalidated when the target leaves the graveyard. This is an engine-wide issue affecting all `GrantFlashback` sources (Snapcaster Mage has the same problem).

**Engine path:**
- `mtg-engine/src/state.rs:209` — `GrantFlashback` stores only `ObjectId`, no `zone_change_count`
- `mtg-engine/src/state.rs:567` — `zone_change_count` incremented on zone change but not checked by GrantFlashback
- `mtg-engine/src/engine.rs:1226-1229` — flashback offering matches by ObjectId only

**Required check:** 8h (continuous effect duration — GrantFlashback incorrectly survives target zone changes)

**Affected cards:**
- Past in Flames
- Snapcaster Mage (same GrantFlashback mechanism)

