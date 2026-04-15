---
id: daybreak_ranger-02
status: deduped
card: Daybreak Ranger
card_file: mtg-engine/src/cards/isd/daybreak_ranger.rs
created: 2026-04-14T21:22:02Z
audit_run_id: 2026-04-14-daybreak_ranger-audit
audit_model: opus
audit_tokens: 12078
audit_duration: 368
deduped_into: merged-dfc-zone-cleanup-01
---

## Audit Finding

**Oracle text:**
> At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

(Back face — but the finding is about the DFC leaving the battlefield after being transformed.)

**Code:**
> `apply_transform` sets `obj.name.clone_from(&back.name)` — helpers.rs:289. `move_object` clears `is_transformed = false` (state.rs:580) but does NOT clear `name` — state.rs:572–583.

**Description:**
When Daybreak Ranger transforms into Nightfall Predator, `apply_transform` (helpers.rs:284–291) sets `obj.name` to "Nightfall Predator" and `obj.subtypes` to `["Werewolf"]`. When the permanent later leaves the battlefield, `move_object` (state.rs:580) clears `is_transformed = false` but does not reset `name` or `subtypes` to front-face values. Per CR 712.8a, a double-faced card outside the battlefield or stack has only the characteristics of its front face. The card retains the name "Nightfall Predator" and subtypes `["Werewolf"]` (missing Human, Archer, Ranger) in the graveyard, hand, exile, and library. This affects cards that reference card names in graveyards (e.g., "return target card named Daybreak Ranger from your graveyard" would fail to find it) and creature type checks in non-battlefield zones.

**Engine path:**
- helpers.rs:262–293 (`apply_transform`)
- state.rs:572–583 (`move_object` cleanup block)

**Required check:** 8a

**Affected cards:**
- Daybreak Ranger / Nightfall Predator
- All double-faced cards that use `apply_transform`
