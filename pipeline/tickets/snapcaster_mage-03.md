---
id: snapcaster_mage-03
status: closed-duplicate
card: Snapcaster Mage
card_file: mtg-engine/src/cards/isd/snapcaster_mage.rs
created: 2026-04-14T20:56:41Z
audit_run_id: 2026-04-14-snapcaster_mage-audit
audit_model: opus
audit_tokens: 19116
audit_duration: 381
duplicate_of: merged-flashback-cost-reduction-02
---

## Audit Finding

**Oracle text:**
> "To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was." (Ruling, 2021-03-19)

**Code:**
> `engine.rs:2219-2227`: When `is_flashback` is true, the cost is taken directly from `GrantFlashback.cost` or `data.flashback_cost` and used as-is.
> `engine.rs:2229-2230`: The non-flashback path applies `effective_spell_cost(&new_state, registry, card_id, &base_cost, player)`.

**Description:**
The CastSpell handler has two cost-resolution branches: non-flashback calls `effective_spell_cost` (which applies cost reductions from permanents like Heartless Summoning), while the flashback branch uses the raw cost without any reduction. Per the ruling and CR 601.2f, cost reductions apply to alternative costs (including flashback costs) the same way they apply to mana costs. If a player controls Heartless Summoning and casts a creature spell via Snapcaster-granted flashback, the {2} reduction should apply but doesn't. The same issue affects static flashback costs (Think Twice, etc.), not just Snapcaster-granted ones.

**Engine path:**
- engine.rs:2219-2227 (flashback cost branch — no `effective_spell_cost` call)
- engine.rs:261-299 (`effective_spell_cost` — the function that should be called)

**Required check:** 8i (casting atomicity — cost determination at 601.2f)

**Affected cards:**
- Snapcaster Mage (dynamically granted flashback)
- All cards with static flashback (Think Twice, Ancient Grudge, etc.)
- Any card with `can_cast_from_graveyard()` if cost reduction should apply
