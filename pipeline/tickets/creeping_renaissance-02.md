---
id: creeping_renaissance-02
status: closed-duplicate
card: Creeping Renaissance
card_file: mtg-engine/src/cards/isd/creeping_renaissance.rs
created: 2026-04-14T21:24:25Z
audit_run_id: 2026-04-14-creeping_renaissance-audit
audit_model: opus
audit_tokens: 25009
audit_duration: 511
duplicate_of: merged-flashback-cost-reduction-01
---

## Audit Finding

**Code:**
> engine.rs:2219-2227:
> ```rust
> } else if is_flashback {
>     let dynamic_fb = new_state.until_end_of_turn.iter()
>         .find_map(|e| if let crate::state::TemporaryEffect::GrantFlashback { target, cost } = e {
>             if *target == *object_id { Some(cost.clone()) } else { None }
>         } else { None });
>     dynamic_fb.unwrap_or_else(|| {
>         data.flashback_cost.expect("flashback cast on card without flashback_cost")
>     })
> }
> ```
> Compare with the non-flashback path at engine.rs:2228-2231:
> ```rust
> } else {
>     let base_cost = data.cost.expect("non-flashback spell must have a mana cost");
>     effective_spell_cost(&new_state, registry, card_id, &base_cost, player)
> };
> ```

**Description:**
Per CR 601.2f and this card's ruling, the total cost of a spell is computed by starting with the mana cost or alternative cost (including flashback), then applying cost increases and reductions. The non-flashback path correctly passes the base cost through `effective_spell_cost` (engine.rs:2230), which applies cost reductions from permanents on the battlefield. The flashback path returns the raw flashback cost directly without any reduction. While no cost reduction in the current card pool applies to sorceries (existing reductions target creature spells via `SpellFilter::CreatureSpells`), the code path deviates from the CR 601.2f procedure. Any future cost reduction that applies to non-creature spells (or all spells) would be silently ignored when casting via flashback.

**Engine path:**
- engine.rs:2217-2231 (cost resolution branches)
- engine.rs:261-291 (effective_spell_cost function, only called for non-flashback)

**Required check:** 8i

**Affected cards:**
- All cards with flashback (Creeping Renaissance, Ancient Grudge, Dream Twist, Forbidden Alchemy, Rolling Temblor, Silent Departure, Spider Spawning, etc.)
