---
id: unburial_rites-01
status: new
card: Unburial Rites
card_file: mtg-engine/src/cards/isd/unburial_rites.rs
created: 2026-04-14T22:50:32Z
audit_run_id: 2026-04-14-unburial_rites-audit
audit_model: opus
audit_tokens: 10537
audit_duration: 4811
---

## Audit Finding

**Oracle text:**
> Flashback {3}{W}

**Code:**
> engine.rs:2217-2231 — The flashback cost path uses the raw flashback cost directly:
> ```rust
> } else if is_flashback {
>     // ...
>     dynamic_fb.unwrap_or_else(|| {
>         data.flashback_cost.expect("flashback cast on card without flashback_cost")
>     })
> } else {
>     let base_cost = data.cost.expect("non-flashback spell must have a mana cost");
>     effective_spell_cost(&new_state, registry, card_id, &base_cost, player)
> };
> ```
> `effective_spell_cost()` is called only in the `else` branch (normal casting). The `is_flashback` branch returns the flashback cost without applying any cost reductions.

**Description:**
Per CR 601.2f and the Scryfall ruling, cost reductions and increases apply to alternative costs (including flashback) just as they do to the normal mana cost. The engine's `effective_spell_cost()` function gathers all `ContinuousEffect::ReduceCost` effects from the caster's permanents and reduces the generic mana portion. However, the flashback casting path at engine.rs:2219 bypasses this function entirely, using the raw flashback cost. If a player controls a permanent with a cost-reduction effect (e.g., a hypothetical "creature spells cost {1} less"), casting Unburial Rites via flashback for {3}{W} would not benefit from the reduction, while casting it normally for {4}{B} would. The same bypass exists in the flashback affordability/autotap computation in `legal_actions` (engine.rs:1263-1291), so the engine is consistently wrong in both the "can I cast this?" check and the actual cost payment.

**Engine path:**
- engine.rs:2219-2227 (cost payment — flashback branch skips `effective_spell_cost`)
- engine.rs:2229-2230 (cost payment — normal branch calls `effective_spell_cost`)
- engine.rs:1263-1291 (affordability check — flashback autotap uses raw cost)
- engine.rs:261 (`effective_spell_cost` definition)

**Required check:** 8i

**Affected cards:**
- Unburial Rites
- All cards with flashback costs (Think Twice, Lingering Souls, Faithless Looting, Devil's Play, Ancient Grudge, Bump in the Night, Dream Twist, etc.)

