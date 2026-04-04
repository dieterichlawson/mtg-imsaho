## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Indestructible
Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
**Type line**: Artifact Creature — Construct
**Status**: ISSUE

### Code issues

- **Lethal-damage + regeneration scenario: winning the coin flip fails to destroy the creature** (`mtg-engine/src/engine.rs` lines 3118–3125, interacting with `mtg-engine/src/triggers.rs` lines 926–931)
  - Oracle text says: `"If you win the flip, destroy that creature."`
  - Second ruling says: `"If the combat damage Creepy Doll deals to a creature is lethal, you'll still flip a coin. If the creature is still on the battlefield (perhaps because it regenerated), it could be destroyed a second time, depending on the coin flip."`
  - Code does: The engine game loop calls `triggers::process_triggers` (which both collects **and resolves** all triggered abilities, including the coin-flip `try_destroy`) **before** it enters the SBA loop. In real MTG, SBAs are checked first: lethal damage would trigger destruction, regeneration replaces that (consuming the shield and clearing damage), and only then does Creepy Doll's triggered ability resolve — at which point there is no shield left and a won flip destroys the creature. In the engine, the sequence is inverted: the trigger fires first, calls `try_destroy` on the creature that still has its shield, the shield is consumed (creature regenerates, damage cleared), and the subsequent SBA pass finds no lethal damage and does nothing. Result: the creature **survives** a won coin flip when it had a regeneration shield and lethal damage, contradicting the oracle text and the explicit ruling.

### Tricky interactions checked

- **Coin flip timing (first ruling: "You don't flip the coin until the ability resolves")**: PASS — `gen_bool(0.5)` is called inside `on_deals_combat_damage_to_creature`, which is called at trigger resolution, not at trigger collection.
- **Mandatory coin flip (no "may")**: PASS — the code always flips; there is no optional skip.
- **`try_destroy` vs `destroy`**: PASS — oracle says "destroy that creature"; `try_destroy` correctly respects indestructible and regeneration.
- **Trigger fires only on creature targets, not player targets**: PASS — `collect_triggers` only creates `CombatDamageToCreature` when `DamageTarget::Object(...)` is present; the `trigger_does_not_fire_on_combat_damage_to_player` test confirms.
- **Indestructible keyword present**: PASS — `Keyword::Indestructible` in `keywords` vec; SBA indestructible check confirmed in `sba.rs`.
- **Trigger description non-empty gate**: PASS — `trigger_description` returns `"flip a coin; if you win, destroy that creature"` (non-empty), so the collection guard at `triggers.rs` line 469 (`if !desc.is_empty()`) does not suppress the trigger.
- **Battlefield guard on `CombatDamageToCreature` resolution (`triggers.rs` line 927)**: Per MTG rules, once a triggered ability is on the stack it resolves independently of its source. The guard `if state.get_object(creature_id).map(|o| o.zone == Zone::Battlefield)` would suppress the flip if Creepy Doll left the battlefield before resolution. In the current engine there is no priority window between combat damage and trigger resolution (engine comment: "No priority in combat damage step for Phase 1"), and Creepy Doll is Indestructible, so this guard is never triggered in practice. Flagged as a latent engine incorrectness but not a currently-observable behaviour failure.
- **Lethal damage with NO regeneration shield**: PASS — trigger fires before SBAs, `try_destroy` finds no shield and no indestructible, creature is destroyed. SBA then finds nothing lethal. Net outcome matches oracle (creature dies regardless of flip because it was lethally hit and the trigger destroys it; if flip lost, SBA kills it instead). Outcome identical to real MTG in this sub-case.
- **Lethal damage WITH regeneration shield (win flip)**: FAIL — see Code Issues above.
- **Non-lethal damage with regeneration shield (win flip)**: PASS — regeneration correctly replaces destruction in both real MTG and the engine; the creature survives because it has a shield, which is the correct result.
- **Creepy Doll's own Indestructible not accidentally applied to the target**: PASS — `try_destroy` is called on `damaged_creature`, not on `self_id`; indestructible check reads the target's keywords.
- **"That creature" referencing the correct damaged creature**: PASS — `damaged_creature` is threaded through from the `CombatDamageToCreature` trigger payload all the way to `on_deals_combat_damage_to_creature`.
- **Redundant battlefield check in the card handler**: PASS (harmless) — `on_deals_combat_damage_to_creature` in `creepy_doll.rs` line 39 re-checks `self_id` zone. Redundant with the engine check at `triggers.rs` line 927 but produces no incorrect behaviour.

### Test coverage

- **Trigger fires on combat damage to creature**: `mtg-engine/tests/creepy_doll.rs:50` — TESTED
- **Trigger does NOT fire on combat damage to player**: `mtg-engine/tests/creepy_doll.rs:78` — TESTED
- **`on_deals_combat_damage_to_creature` can eventually destroy a target (probabilistic)**: `mtg-engine/tests/creepy_doll.rs:104` — TESTED
- **Indestructible keyword present**: `mtg-engine/tests/creepy_doll.rs:40` — TESTED
- **Correct `TriggerKind` (not Blocks/BecomesBlocked)**: `mtg-engine/tests/creepy_doll.rs:22` — TESTED
- **Coin flip happens at resolution not at trigger collection**: NOT TESTED
- **Lethal damage + regeneration shield scenario (win flip)**: NOT TESTED
- **Lethal damage with no shield (win flip destroys, lose flip SBA kills)**: NOT TESTED
- **Non-lethal damage + regeneration shield (creature regenerates, survives)**: NOT TESTED
- **Trigger resolves (or does not) when Creepy Doll leaves battlefield before resolution**: NOT TESTED
