---
id: heartless_summoning-02
status: closed-duplicate
card: Heartless Summoning
card_file: mtg-engine/src/cards/isd/heartless_summoning.rs
created: 2026-04-14T21:29:30Z
audit_run_id: 2026-04-14-heartless_summoning-audit
audit_model: opus
audit_tokens: 19813
audit_duration: 410
duplicate_of: merged-flashback-cost-reduction-02
---

## Audit Finding

**Oracle text:**
> Creature spells you cast cost {2} less to cast.

**Code:**
> engine.rs:1235-1240 — legal_actions cast-from-graveyard path uses raw `data.cost`:
> ```
> None => if cast_from_gy {
>     // Cast from graveyard uses normal mana cost.
>     match &data.cost {
>         Some(c) => c,
>         None => continue,
>     }
> ```
> But engine.rs:2229-2230 (submit_action) correctly applies `effective_spell_cost`:
> ```
> let base_cost = data.cost.expect("non-flashback spell must have a mana cost");
> effective_spell_cost(&new_state, registry, card_id, &base_cost, player)
> ```

**Description:**
When a creature with `can_cast_from_graveyard()` (e.g., Skaab Ruinator) is in the graveyard, the `legal_actions` function uses the raw mana cost for the affordability/autotap check, without applying cost reductions from Heartless Summoning. However, `submit_action` correctly applies `effective_spell_cost` to the same spell. This mismatch means a cast-from-graveyard creature spell may not appear in the player's legal actions (shown as unaffordable) even when Heartless Summoning's {2} reduction would make it affordable. For example, Skaab Ruinator costs {1}{U}{U}; with Heartless Summoning the effective cost is {U}{U}, but legal_actions checks against the full {1}{U}{U}.

**Engine path:**
- engine.rs:1235-1240 (legal_actions graveyard cast — raw cost, missing reduction)
- engine.rs:2229-2230 (submit_action graveyard cast — correctly reduced)

**Required check:** 8i

**Affected cards:**
- Heartless Summoning + Skaab Ruinator (creature with can_cast_from_graveyard)
- Any ReduceCost effect + any cast-from-graveyard creature
