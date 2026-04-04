## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues

- **Engine bug: `SpellCast` trigger dispatch restricted to instant/sorcery** — `mtg-engine/src/triggers.rs` lines 644–675
  - Oracle text says: `Whenever you cast a spell from your graveyard`
  - Code does: `let is_instant_sorcery = ... .map(|d| d.card_types.iter().any(|ct| matches!(ct, crate::types::CardType::Instant | crate::types::CardType::Sorcery))).unwrap_or(false); if is_instant_sorcery { /* fire SpellCastWatch triggers */ }` — Burning Vengeance's `SpellCastWatch` trigger is never collected for non-instant/sorcery spells. Skaab Ruinator is a Creature that can be cast from the graveyard (via `can_cast_from_graveyard()`) and is NOT an instant or sorcery, so the trigger is silently dropped. The oracle text places no type restriction on "a spell."

- **Card bug: checks `cast_with_flashback` rather than "cast from graveyard"** — `mtg-engine/src/cards/isd/burning_vengeance.rs` lines 48–53
  - Oracle text says: `Whenever you cast a spell from your graveyard`
  - Code does: `let cast_from_gy = state.get_object(spell_id).map(|o| o.cast_with_flashback).unwrap_or(false); if !cast_from_gy { return; }` — The variable is named `cast_from_gy` but reads the `cast_with_flashback` field. In `engine.rs` lines 1491–1492 and 1636–1638, `cast_with_flashback` is only set to `true` for the flashback path (`is_flashback = in_graveyard && !is_cast_from_graveyard`). When Skaab Ruinator is cast from the graveyard, the engine takes the `is_cast_from_graveyard = true` branch and **never sets `cast_with_flashback`**. So even if the engine dispatch bug above were fixed, Burning Vengeance would still incorrectly skip the trigger for Skaab Ruinator.

- **Log message logged before target is chosen, and describes "opponent" inaccurately** — `mtg-engine/src/cards/isd/burning_vengeance.rs` lines 67–69
  - Oracle text says: `this enchantment deals 2 damage to any target`
  - Code does: `state.log(..., format!("Burning Vengeance deals 2 damage to opponent (flashback spell cast)"))` — This log line is written after `present_target_choice`, which (when there are multiple targets) sets `awaiting_action` and returns without yet applying damage. The log asserts damage was dealt to the opponent before the player has even chosen a target. Additionally, "opponent" is wrong — the target may be any creature or any player (including your own creatures, planeswalkers-as-targets if supported, or even yourself).

### Tricky interactions checked

- **"a spell" vs. "an instant or sorcery spell"**: FAIL — The engine's `SpellCast` dispatch in `triggers.rs` guards on `is_instant_sorcery`; spells that are neither instant nor sorcery (e.g., Skaab Ruinator cast from graveyard) do not cause `SpellCastWatch` to be collected at all.
- **Flashback path sets `cast_with_flashback` correctly**: PASS — For cards with `flashback_cost` (or dynamically granted flashback via Past in Flames / Snapcaster Mage), `engine.rs` sets `obj.cast_with_flashback = true` at line 1637. Burning Vengeance's check works for standard flashback instants/sorceries.
- **`can_cast_from_graveyard()` path does NOT set `cast_with_flashback`**: FAIL — `engine.rs` line 1636 is inside `if is_flashback`, which is false when `is_cast_from_graveyard = true`. Skaab Ruinator cast from the graveyard gets `cast_with_flashback = false`, so the card-level check in `burning_vengeance.rs` returns early.
- **"you cast" — controller-only restriction**: PASS — `on_spell_cast` checks `if caster != controller { return; }` at line 44, correctly ignoring opponents' spells.
- **Trigger timing — resolves before the spell**: PASS — The `SpellCast` event is emitted while the spell is on the stack; `collect_triggers` adds the `SpellCastWatch` trigger on top of the stack before the spell, so the trigger resolves first per standard MTG rules.
- **Ruling: activated abilities (unearth, Reassembling Skeleton) do not trigger**: PASS — The trigger only listens for `GameEvent::SpellCast`; no activated-ability events in the engine match this, so activated graveyard abilities do not mistakenly trigger Burning Vengeance.
- **"any target" includes all creatures and both players**: PASS — `any_targets()` in helpers.rs collects all battlefield creatures plus all players, which matches "any target."
- **Target choice is optional vs. mandatory**: PASS — `present_target_choice` is called with `optional: false`, consistent with "deals 2 damage to any target" being a mandatory effect. (Note: there must be at least one legal target for the trigger to be put on the stack; the `if targets.is_empty() { return; }` guard handles the edge case where no targets exist, though the rules would say the ability fizzles.)
- **Burning Vengeance leaves battlefield before trigger resolves**: PASS — `on_spell_cast` first verifies Burning Vengeance is still on the battlefield (`o.zone == Zone::Battlefield`) before proceeding; and `resolve_next_trigger` also re-verifies (`watcher must be on battlefield`) before calling the behavior, so the ability correctly doesn't fire if the enchantment has left.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Triggers on flashback instant/sorcery cast from graveyard: `tests/tier12_cards.rs:282` (`burning_vengeance_triggers_on_flashback`) — TESTED
- Does not trigger on normal (non-graveyard) spell casts: `tests/tier12_cards.rs:329` (`burning_vengeance_ignores_non_flashback`) — TESTED
- Does not trigger when opponent casts a flashback spell: NOT TESTED
- Triggers when Skaab Ruinator is cast from the graveyard (creature spell, `can_cast_from_graveyard`): NOT TESTED — and would fail due to the two bugs above
- Trigger resolves before the flashback spell: NOT TESTED
- Ruling: activated graveyard abilities do not trigger (unearth, Reassembling Skeleton): NOT TESTED
- Past in Flames + Burning Vengeance interaction (dynamically granted flashback): NOT TESTED
- Target choice includes creatures (not just players): NOT TESTED
- Burning Vengeance itself leaves battlefield before trigger resolves (ability should fizzle): NOT TESTED
