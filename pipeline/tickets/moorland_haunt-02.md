---
id: moorland_haunt-02
status: closed-duplicate
card: Moorland Haunt
card_file: mtg-engine/src/cards/isd/moorland_haunt.rs
created: 2026-04-14T21:31:23Z
audit_run_id: 2026-04-14-moorland_haunt-audit
audit_model: opus
audit_tokens: 13727
audit_duration: 355
duplicate_of: merged-activation-cost-variants-01
---

## Audit Finding

**Oracle text:**
> {W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.

**Code:**
> In `on_activate_ability` (moorland_haunt.rs:76-108), when multiple creatures are in the graveyard, the code calls `present_target_choice` (moorland_haunt.rs:101-106) which sets `awaiting_action` and returns control to the game loop. The exile and token creation are deferred to `PendingEffect::ExileFromGraveyardAndCreateToken` (engine.rs:3946-3956).

**Description:**
The exile is a cost of the activated ability (it appears before the colon in the oracle text). Per CR 602.2, all costs are paid atomically during activation. The engine pays mana ({W}{U}) and tap costs at engine.rs:2646-2653 before calling `on_activate_ability`. When multiple creatures are in the graveyard, `on_activate_ability` defers the exile to a player prompt via `present_target_choice`. This creates an intermediate game state where the mana and tap costs are paid but the exile cost has not been paid — a "half-activated" ability. Per CR 602.2, the player should choose which creature to exile as part of announcing the ability (before any costs are paid), and then all costs (mana, tap, exile) should be paid atomically. The `ActivatedAbilityDef` struct lacks a field for "exile from graveyard" costs, so the card implementation is forced to handle it in `on_activate_ability`, but this architectural constraint produces incorrect sequencing. Note: the single-creature case (moorland_haunt.rs:86-98) handles this atomically since no choice is needed.

**Engine path:**
- mtg-engine/src/engine.rs:2646-2653 (mana and tap costs paid)
- mtg-engine/src/engine.rs:2718-2719 (on_activate_ability called)
- mtg-engine/src/cards/isd/moorland_haunt.rs:99-107 (choice deferred via present_target_choice)
- mtg-engine/src/engine.rs:3946-3956 (deferred effect handler)

**Required check:** 8i

**Affected cards:**
- Moorland Haunt
- Any card with an exile-from-graveyard activation cost where multiple candidates exist
