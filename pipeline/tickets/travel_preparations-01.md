---
id: travel_preparations-01
status: new
card: Travel Preparations
card_file: mtg-engine/src/cards/isd/travel_preparations.rs
created: 2026-04-15T03:48:16Z
audit_run_id: 2026-04-14-travel_preparations-audit
audit_model: opus
audit_tokens: 13605
audit_duration: 327
---

## Audit Finding

**Oracle text:**
> Flashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)

Combined with ruling [2021-03-19]: "To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions."

**Code:**
> At engine.rs:2217-2231, the flashback cost path:
> ```
> } else if is_flashback {
>     let dynamic_fb = new_state.until_end_of_turn.iter()
>         .find_map(|e| if let crate::state::TemporaryEffect::GrantFlashback { target, cost } = e {
>             if *target == *object_id { Some(cost.clone()) } else { None }
>         } else { None });
>     dynamic_fb.unwrap_or_else(|| {
>         data.flashback_cost.expect("flashback cast on card without flashback_cost")
>     })
> } else {
>     let base_cost = data.cost.expect("non-flashback spell must have a mana cost");
>     effective_spell_cost(&new_state, registry, card_id, &base_cost, player)
> };
> ```

**Description:**
Per CR 601.2f, the total cost of a spell starts with the mana cost or alternative cost (such as a flashback cost), then adds cost increases and subtracts cost reductions. The non-flashback path correctly calls `effective_spell_cost` to apply cost reductions, but the flashback path returns the raw flashback cost directly without any cost modification. This means if a cost-reduction effect is active (e.g., a creature like Thunderscape Familiar reducing spell costs, or any future Innistrad-set cost reducer), it would correctly reduce the normal casting cost of Travel Preparations but would NOT reduce the flashback cost {1}{W}. The same gap exists in the `legal_actions` affordability check at engine.rs:1263-1296, where the flashback cost is used raw for autotap computation. Both the affordability check and the actual payment skip `effective_spell_cost` for flashback, so they're internally consistent but both wrong per the rules.

**Engine path:**
- engine.rs:2219 (flashback cost resolution during CastSpell execution)
- engine.rs:2261 (effective_spell_cost definition — only called for non-flashback)
- engine.rs:1263-1296 (flashback autotap in legal_actions — also skips cost reduction)

**Required check:** 8i

**Affected cards:**
- Travel Preparations
- All cards with flashback_cost (Ancient Grudge, Bump in the Night, Devil's Play, Dream Twist, Feeling of Dread, Forbidden Alchemy, Geistflame, Gnaw to the Bone, Memory's Journey, Nightbird's Clutches, Rolling Temblor, Silent Departure, Think Twice, Unburial Rites, and any card granted flashback by Past in Flames or Snapcaster Mage)

## Tests

### flashback_cost_reduction_applied
Source ticket: (new)
Implementation: (not yet written)
Scenario: Set up a continuous effect that reduces sorcery spell costs by {1} (or a generic cost reducer). Place Travel Preparations in the graveyard. Verify that the flashback cost is reduced from {1}{W} to {W} — i.e., the spell can be cast with only {W} in pool, and the player is not charged {1}{W}.

