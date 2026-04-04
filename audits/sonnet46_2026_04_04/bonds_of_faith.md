## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues

- **"as long as it's a Human" condition is snapshotted at ETB, never re-evaluated** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` lines 39–69
  - Oracle text says: `"Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block."`
  - Code does: `on_enter_battlefield` checks the Human subtype once at entry time and writes a fixed `instance_continuous_effects` (`[ModifyPT { +2/+2 }]` for Human, `[PreventAttack, PreventBlock]` for non-Human) that is never updated. `continuous_pt_mods` and `has_continuous_effect` in `state.rs` consume `instance_continuous_effects` verbatim without re-evaluating the Human condition. `apply_transform` in `helpers.rs` updates `obj.subtypes` on the enchanted creature but leaves Bonds of Faith's `instance_continuous_effects` untouched. Therefore: a Human that transforms into a non-Human (e.g., Village Ironsmith → Ironfang) continues to receive +2/+2 and is not prevented from attacking or blocking — the opposite of what the oracle text requires.

- **Human subtype check at ETB only inspects registry data, missing object-level subtypes (tokens)** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` lines 43–46
  - Oracle text says: `"as long as it's a Human"`
  - Code does: `state.get_object(target_id).and_then(|o| registry.card_data(o.card_id)).map(|d| d.subtypes.iter().any(|s| s == "Human")).unwrap_or(false)` — tokens use `card_id = CardId(0)` for which `registry.card_data()` returns `None`, so `is_human` is always `false` for tokens regardless of their actual subtypes. Human tokens would incorrectly receive `[PreventAttack, PreventBlock]` instead of `+2/+2`. Compare with `check_condition` / `matches_filter` in `state.rs` which correctly checks both `o.subtypes` and `registry.card_data()`.

- **`oracle_text` field is missing the "Enchant creature" first line** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` line 25
  - Oracle text says: `"Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block."`
  - Code does: `oracle_text: "Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.".into()` — the "Enchant creature" first line is absent. Other auras in the same set (e.g., `dead_weight.rs`, `wreath_of_geists.rs`, `curiosity.rs`, `sensory_deprivation.rs`, `claustrophobia.rs`) all include this prefix; Bonds of Faith does not.

### Tricky interactions checked

- **"as long as" requires continuous re-evaluation**: FAIL — effects are snapshotted once at ETB and never reconsidered; the engine's `ContinuousEffect` enum has no conditional-PT variant that would support continuous re-evaluation.
- **Transforming Human (e.g., Village Ironsmith → Ironfang) while enchanted**: FAIL — `apply_transform` updates `obj.subtypes` on the creature but Bonds of Faith's `instance_continuous_effects` is left unchanged, so the +2/+2 persists after transformation instead of switching to the attack/block restriction.
- **Non-Human becoming Human while enchanted** (e.g., via Shields of Velis Vel): FAIL — same root cause in the other direction; `[PreventAttack, PreventBlock]` remains in `instance_continuous_effects` even after the creature gains the Human subtype.
- **Human token enchanted by Bonds of Faith**: FAIL — ETB subtype check only reads `registry.card_data()` which returns `None` for tokens (sentinel `CardId(0)`), so Human tokens are misidentified as non-Human and receive the wrong effect.
- **Non-Human non-token at ETB (static case)**: PASS — `instance_continuous_effects` correctly set to `[PreventAttack, PreventBlock]`; `can_attack`/`can_block` in `state.rs` both check these effects and also have a legacy `instance_oracle_text` fallback.
- **Human (non-token) at ETB (static case)**: PASS — `instance_continuous_effects` correctly set to `[ModifyPT { +2/+2 }]`; `continuous_pt_mods` in `state.rs` applies it.
- **Declared attacker/blocker losing Human status mid-combat**: Partially addressed by ruling "Once declared as attacking/blocking, losing Human status won't remove it from combat." The code does not model this ruling explicitly, but since `can_attack`/`can_block` are only checked at declaration time, and the effects are snapshotted anyway, this specific interaction happens not to be wrong — but only by accident, and the +2/+2 loss on transformation is still broken per the ruling ("It will lose the +2/+2 bonus, however.").
- **Aura falling off when enchanted creature leaves battlefield**: PASS — `resolve_aura` in `helpers.rs` handles this; engine's SBA would detach the aura if the creature leaves the battlefield.
- **`move_spell_after_resolve` vs `move_object(Zone::Graveyard)`**: PASS — `on_resolve` calls `crate::cards::helpers::resolve_aura` which uses `move_spell_after_resolve` for the fizzle case.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Human at ETB gets +2/+2: `mtg-engine/tests/card_mechanics.rs` (`bonds_of_faith_buffs_human`) and `mtg-engine/tests/bug_fixes.rs` (`bonds_of_faith_gives_plus_two_to_human`)
- Non-Human at ETB gets can't attack/block: `mtg-engine/tests/innistrad_cards.rs` (`bonds_of_faith_prevents_attack_and_block`), `mtg-engine/tests/card_mechanics.rs` (`bonds_of_faith_locks_non_human`), `mtg-engine/tests/bug_fixes.rs` (`bonds_of_faith_locks_non_human`)
- Human transforms to non-Human after Bonds of Faith ETB (ruling 2011-09-22: loses +2/+2): NOT TESTED
- Non-Human gains Human after Bonds of Faith ETB: NOT TESTED
- Human token enchanted by Bonds of Faith gets +2/+2: NOT TESTED
- Declared attacker/blocker loses Human in mid-combat (ruling 2011-09-22): NOT TESTED
