---
id: grimoire_of_the_dead-02
status: closed-duplicate
card: Grimoire of the Dead
card_file: mtg-engine/src/cards/isd/grimoire_of_the_dead.rs
created: 2026-04-14T20:57:12Z
audit_run_id: 2026-04-14-grimoire_of_the_dead-audit
audit_model: opus
audit_tokens: 16027
audit_duration: 412
duplicate_of: merged-zone-cleanup-characteristics-01
---

## Audit Finding

**Oracle text:**
> They're black Zombies in addition to their other colors and types.

**Code:**
> grimoire_of_the_dead.rs:162-167:
> ```
> if !obj.subtypes.contains(&"Zombie".into()) {
>     obj.subtypes.push("Zombie".into());
> }
> if !obj.colors.contains(&Color::Black) {
>     obj.colors.push(Color::Black);
> }
> ```
> state.rs:572-583 (zone-change cleanup): `subtypes` and `colors` are NOT in the list of fields cleared when an object leaves the battlefield.

**Description:**
The ability adds "Zombie" to `obj.subtypes` and `Color::Black` to `obj.colors` directly on the object. Per CR 611.2a, this one-shot effect lasts indefinitely while the object remains on the battlefield. However, per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. The engine's `move_object` cleanup (state.rs:572-583) does NOT clear `subtypes` or `colors` when an object leaves the battlefield. If a creature reanimated by Grimoire later dies and is returned to the battlefield by another effect, it would incorrectly retain the Zombie subtype and black color from the Grimoire's previous effect.

**Engine path:**
- grimoire_of_the_dead.rs:162-167 (direct object mutation)
- state.rs:572-583 (cleanup omits subtypes, colors)

**Required check:** 8a

**Affected cards:**
- Grimoire of the Dead
- Any card that adds subtypes or colors to objects at runtime (engine-wide: zone-change cleanup gap)
