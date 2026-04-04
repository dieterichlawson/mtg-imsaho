## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
**Type line**: Creature — Devil
**Status**: ISSUE

### Code issues

- Upkeep trigger fires spuriously on the stack during the opponent's upkeep
  - Oracle text says: `At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.`
  - Code does: `triggers.rs` lines 597–639 dispatch `UpkeepTrigger` for **all** battlefield permanents that have an Upkeep trigger description whenever any `StepStarted { step: Upkeep }` event fires, with no check that `controller == active_player`. Charmbreaker Devils' upkeep trigger is therefore collected and pushed onto the stack during the opponent's upkeep. `on_upkeep` (charmbreaker_devils.rs line 51) guards with `if state.active_player != controller { return; }`, so the effect is suppressed, but the trigger is still on the stack, giving players a spurious priority window to respond to a trigger that should never have existed.

- SpellCast trigger fires spuriously on the stack when the opponent casts an instant or sorcery
  - Oracle text says: `Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.`
  - Code does: `triggers.rs` lines 644–676 dispatch `SpellCastWatch` for **all** battlefield permanents that have a SpellCast trigger description whenever any instant or sorcery is cast by any player. Charmbreaker Devils' SpellCast trigger is therefore collected and pushed onto the stack when the opponent casts an instant or sorcery. `on_spell_cast` (charmbreaker_devils.rs line 81) guards with `if caster != controller { return; }`, so the effect is suppressed, but the trigger is still on the stack, giving players a spurious priority window to respond to a trigger that the oracle text restricts to the controller's own casts.

### Tricky interactions checked

- "At the beginning of your upkeep" — fires only on controller's upkeep: FAIL (trigger appears on stack during opponent's upkeep as documented above)
- "Whenever you cast" — fires only when controller casts: FAIL (trigger appears on stack when opponent casts an instant/sorcery as documented above)
- Random selection of instant/sorcery from graveyard — card chosen at random as ability resolves (not targeted): PASS (uses `SliceRandom::shuffle` on candidates vec, picks index 0; non-targeted so cards added to graveyard in response are eligible)
- +4/+0 until end of turn cleanup — cleared at cleanup step: PASS (`state.until_end_of_turn_effects.clear()` at engine.rs line 3021 during `Step::Cleanup`)
- Upkeep trigger when graveyard is empty — does nothing: PASS (`if !candidates.is_empty()` check at charmbreaker_devils.rs line 64)
- SpellCast trigger filters to instants/sorceries only — non-instant/non-sorcery spells do not trigger: PASS (dispatch at triggers.rs line 650 gates on `is_instant_sorcery`)
- "You" in "you cast" — correctly excludes opponent's spells from applying the +4/+0 effect: PASS (on_spell_cast line 81 returns if `caster != controller`)
- +4/+0 stacks with multiple spell casts in a turn: PASS (each spell cast pushes an independent `UntilEndOfTurnEffect`; `effective_power` sums all matching entries)

### Test coverage

- Upkeep return-to-hand ability fires on controller's upkeep: NOT TESTED
- Upkeep trigger does NOT fire (i.e., is absent from stack) on opponent's upkeep: NOT TESTED
- Random card selection from among multiple instants/sorceries in graveyard: NOT TESTED
- Cards added to graveyard in response to the upkeep trigger are eligible to be returned: NOT TESTED
- SpellCast trigger fires when controller casts an instant: `mtg-engine/tests/tier7_cards.rs` line 247 (`charmbreaker_devils_plus4_on_spell_cast`)
- SpellCast trigger does NOT fire (i.e., absent from stack) when opponent casts an instant/sorcery: NOT TESTED
- SpellCast trigger does NOT fire when controller casts a non-instant non-sorcery: NOT TESTED
- +4/+0 expires at end of turn: NOT TESTED
- Multiple spell casts stack the bonus: NOT TESTED
