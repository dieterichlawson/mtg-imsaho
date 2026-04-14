---
id: army_of_the_damned-02
status: new
card: Army of the Damned
card_file: mtg-engine/src/cards/isd/army_of_the_damned.rs
created: 2026-04-14T20:46:53Z
audit_run_id: 2026-04-14-army_of_the_damned-audit
audit_model: opus
audit_tokens: 21975
audit_duration: 465
---

## Audit Finding

**Oracle text:**
> Flashback {7}{B}{B}{B}

**Code:**
> engine.rs:2217-2231 — Flashback casts return the raw flashback cost directly (`data.flashback_cost`), bypassing `effective_spell_cost` which applies cost reductions. Normal (non-flashback) casts at line 2229-2230 route through `effective_spell_cost`.

**Description:**
Per CR 601.2f and the ruling, cost increases and reductions apply to all spells regardless of whether the base cost is the mana cost or an alternative cost like flashback. The engine's flashback casting path at engine.rs:2219-2227 returns the raw flashback cost without applying `effective_spell_cost`. Currently `effective_spell_cost` only reduces creature spell costs (via `SpellFilter::CreatureSpells` and `SpellFilter::CreatureWithSubtype` at engine.rs:288-291), so no reducer affects this sorcery. However, the engine path is structurally incorrect — if a future cost reducer targeted sorceries or all spells, flashback casts would ignore it. This is an engine-wide issue affecting all flashback cards.

**Engine path:**
- engine.rs:2219-2227 (flashback cost path — raw cost, no reduction)
- engine.rs:2229-2230 (normal cast path — goes through `effective_spell_cost`)
- engine.rs:261-305 (`effective_spell_cost` — applies cost reduction)

**Required check:** 8i, 8j

**Affected cards:**
- Army of the Damned
- All cards with flashback_cost in the ISD set

