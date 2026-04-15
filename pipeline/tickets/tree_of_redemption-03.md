---
id: tree_of_redemption-03
status: closed-duplicate
card: Tree of Redemption
card_file: mtg-engine/src/cards/isd/tree_of_redemption.rs
created: 2026-04-14T21:48:34Z
audit_run_id: 2026-04-14-tree_of_redemption-audit
audit_model: opus
audit_tokens: 9193
audit_duration: 1139
duplicate_of: merged-activated-no-stack-02
---

## Audit Finding

**Oracle text:**
> {T}: Exchange your life total with this creature's toughness.

**Code:**
> `engine.rs:2717-2719`:
> ```
> // Non-X ability: fire the effect immediately.
> if let Some(behavior) = registry.get(behavior_card_id) {
>     behavior.on_activate_ability(&mut new_state, *object_id, *ability_index, targets, registry);
> }
> ```

**Description:**
Per CR 602.2, activated abilities go on the stack and resolve when they receive priority. Opponents can respond to the ability before it resolves (e.g., destroying Tree with an instant). The first ruling explicitly describes this: the exchange fails if Tree isn't on the battlefield at resolution. However, the engine fires `on_activate_ability` immediately upon activation, bypassing the stack entirely. There is no window for opponents to respond between activation and resolution. This means the ruling's scenario — Tree leaving before its ability resolves — cannot occur. This is a known engine-wide limitation affecting all activated abilities.

**Engine path:**
- engine.rs:2717-2719 (immediate execution of on_activate_ability)

**Required check:** 8c

**Affected cards:**
- Tree of Redemption
- All cards with activated abilities (engine-wide)
