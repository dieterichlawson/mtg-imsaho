---
id: tree_of_redemption-02
status: new
card: Tree of Redemption
card_file: mtg-engine/src/cards/isd/tree_of_redemption.rs
created: 2026-04-14T21:48:34Z
audit_run_id: 2026-04-14-tree_of_redemption-audit
audit_model: opus
audit_tokens: 9193
audit_duration: 1139
---

## Audit Finding

**Oracle text:**
> {T}: Exchange your life total with this creature's toughness.

**Code:**
> `tree_of_redemption.rs:62`: `state.get_player_mut(controller).life = current_toughness;`
> `tree_of_redemption.rs:63-67`: Emits `GameEvent::LifeChanged { player, old, new_life }` — a generic event with no gain/loss distinction.

**Description:**
Per CR 118.6, exchanging life totals is implemented as gaining or losing the necessary amount of life. If a player's life goes from 20 to 13, they lose 7 life; if from 5 to 13, they gain 8 life. Effects that trigger on or interact with life gain (e.g., "whenever you gain life") or life loss (e.g., "whenever you lose life") must interact with this exchange accordingly. The implementation directly assigns the new life total without computing or emitting a gain/loss event. The engine only has `LifeChanged` events (events.rs:35), not separate `LifeGained`/`LifeLost` events, and the trigger system (triggers.rs) does not handle `LifeChanged` at all. This means life-gain and life-loss triggers do not fire from this exchange (or from any life change in the engine — this is an engine-wide gap).

**Engine path:**
- tree_of_redemption.rs:62 (direct life assignment)
- events.rs:35 (LifeChanged event definition — no gain/loss distinction)
- triggers.rs (no handler for LifeChanged events)

**Required check:** 8j (ruling interaction)

**Affected cards:**
- Tree of Redemption
- Tree of Perdition (same exchange mechanic)
- Any card that interacts with life gain/loss triggers (engine-wide: no card emits gain/loss events)

