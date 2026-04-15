---
id: creeping_renaissance-01
status: closed-duplicate
card: Creeping Renaissance
card_file: mtg-engine/src/cards/isd/creeping_renaissance.rs
created: 2026-04-14T21:24:25Z
audit_run_id: 2026-04-14-creeping_renaissance-audit
audit_model: opus
audit_tokens: 25009
audit_duration: 511
duplicate_of: merged-zone-cleanup-characteristics-02
---

## Audit Finding

**Oracle text:**
> Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.

**Code:**
> engine.rs:3089-3101:
> ```rust
> let to_return: Vec<ObjectId> = new_state.objects_in_zone(Zone::Graveyard, *controller)
>     .iter()
>     .filter(|o| {
>         // Check object's own card_types first, fall back to registry
>         if o.card_types.is_empty() {
>             registry.card_data(o.card_id)
>                 .is_some_and(|d| d.card_types.contains(&card_type))
>         } else {
>             o.card_types.contains(&card_type)
>         }
>     })
> ```

**Description:**
Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. In the graveyard, a card should have only its printed characteristics. The filter checks `o.card_types` first and only falls back to the registry when empty. For most cards `card_types` is empty (initialized as `Vec::new()` in `create_object` at state.rs:318), so the registry fallback fires correctly. However, copy effects explicitly set `card_types` at runtime (state.rs:687 for entering-as-copy, engine.rs:3760 for mid-game copy), and `move_object` does not clear `card_types` when leaving the battlefield (state.rs:572-583 — the cleanup block clears tapped, summoning_sick, damage_marked, counters, is_transformed, etc., but NOT card_types, subtypes, keywords, name, colors, power, or toughness). A creature that entered via a copy effect and then died retains its copied card_types in the graveyard, potentially causing Creeping Renaissance to include or exclude it incorrectly. The correct approach is to always consult the registry for objects in the graveyard.

**Engine path:**
- engine.rs:3089-3101 (card type filter)
- state.rs:572-583 (zone-change cleanup — card_types not cleared)
- state.rs:687 (copy effect sets card_types)
- engine.rs:3760 (copy effect sets card_types)

**Required check:** 8d (combined with 8a zone-change cleanup gap)

**Affected cards:**
- Creeping Renaissance
- Any future card that checks card types for objects in the graveyard
