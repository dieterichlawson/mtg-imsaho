---
id: grimoire_of_the_dead-04
status: closed-duplicate
card: Grimoire of the Dead
card_file: mtg-engine/src/cards/isd/grimoire_of_the_dead.rs
created: 2026-04-14T20:57:12Z
audit_run_id: 2026-04-14-grimoire_of_the_dead-audit
audit_model: opus
audit_tokens: 16027
audit_duration: 412
duplicate_of: merged-activation-cost-variants-01
---

## Audit Finding

**Oracle text:**
> {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.

**Code:**
> grimoire_of_the_dead.rs:55-64 (ActivatedAbilityDef):
> ```
> cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
> requires_tap: true,
> sacrifice_cost: SacrificeCost::None,
> ```
> grimoire_of_the_dead.rs:91-129 (on_activate_ability): discard handled here, after engine has already paid {1} and tapped the Grimoire.

**Description:**
"Discard a card" is part of the activation cost per CR 602.2b — it should be paid at 602.2g alongside {1} and {T}, before the ability is considered activated. The `ActivatedAbilityDef` struct has no field for discard costs, so the discard is deferred to `on_activate_ability`. For the multi-card case (lines 118-129), the engine opens a `ChooseCardFromHand` prompt after the {1} and tap costs are already paid, leaving the ability in a half-activated state (mana spent, Grimoire tapped, but discard not yet performed). Per CR 602.2, all costs must be determined and paid before the ability is considered activated. The engine currently resolves activated abilities immediately (no stack), which masks the timing issue in practice, but the cost ordering is still non-compliant.

**Engine path:**
- engine.rs:2645-2653 (mana + tap paid)
- engine.rs:2717-2719 (on_activate_ability called)
- grimoire_of_the_dead.rs:120-128 (deferred discard choice)
- engine.rs:3037-3051 (ChooseCardFromHand resolution)

**Required check:** 8i

**Affected cards:**
- Grimoire of the Dead
- Any card with discard-as-cost activated abilities (engine-wide: ActivatedAbilityDef lacks discard cost support)
