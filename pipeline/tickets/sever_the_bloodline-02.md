---
id: sever_the_bloodline-02
status: new
card: Sever the Bloodline
card_file: mtg-engine/src/cards/isd/sever_the_bloodline.rs
created: 2026-04-15T03:47:14Z
audit_run_id: 2026-04-14-sever_the_bloodline-audit
audit_model: opus
audit_tokens: 20080
audit_duration: 488
---

## Audit Finding

**Oracle text:**
> Flashback {5}{B}{B}

Per ruling: "To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was."

**Code:**
> engine.rs:2219-2227 — flashback cast path uses raw cost:
> ```
> } else if is_flashback {
>     dynamic_fb.unwrap_or_else(|| {
>         data.flashback_cost.expect("flashback cast on card without flashback_cost")
>     })
> }
> ```
> Versus the normal cast path at engine.rs:2229-2230:
> ```
> let base_cost = data.cost.expect("non-flashback spell must have a mana cost");
> effective_spell_cost(&new_state, registry, card_id, &base_cost, player)
> ```

**Description:**
The flashback cast path (engine.rs:2219-2227) takes the raw `data.flashback_cost` without calling `effective_spell_cost()`, which applies cost reductions from `ContinuousEffect::ReduceCost` effects (engine.rs:261-291). Per CR 601.2e-f, cost reductions apply regardless of whether the base cost is a mana cost or an alternative cost (like flashback). If a player controls a permanent with a relevant `ReduceCost` effect, the flashback cost of {5}{B}{B} should be reduced but is not. The `legal_actions` enumeration for flashback (engine.rs:1263-1266) also skips cost reduction when computing autotap, so the affordability check is consistent with the incorrect behavior — the spell may not appear castable even when a cost reducer makes it affordable.

**Engine path:**
- engine.rs:2219-2227 (flashback cost selection — missing `effective_spell_cost`)
- engine.rs:2229-2230 (normal cost path — correctly applies `effective_spell_cost`)
- engine.rs:261-291 (`effective_spell_cost` — handles cost reductions)
- engine.rs:1263-1266 (flashback autotap in `legal_actions` — also skips reduction)

**Required check:** 8i

**Affected cards:**
- Sever the Bloodline
- All cards with flashback (engine-wide: the flashback cast path is shared)

## Tests

### sever_flashback_cost_reduction_applies
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place a permanent on the battlefield that grants `ContinuousEffect::ReduceCost` for the caster (e.g., a familiar reducing spell costs by {1}). Put Sever the Bloodline in the graveyard. Verify that casting it via flashback costs {4}{B}{B} (reduced from {5}{B}{B}), not the unreduced {5}{B}{B}.

