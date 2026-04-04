## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: `{1}{W}, Sacrifice this creature: Creatures you control get +1/+1 until end of turn.`
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Sacrifice-as-cost before resolution**: Selfless Cathar is sacrificed (`destruction::sacrifice` → `move_object(Zone::Graveyard)`) at line 1748 of `engine.rs` before `on_activate_ability` is called at line 1802. The card then reads `state.get_object(object_id)` to recover the controller from the graveyard object, which works because `move_object` keeps the object in `state.objects` (only zone is changed). The creature filter then excludes the cathar itself because `obj.zone == Zone::Battlefield` is false for the graveyard object. Correct — the cathar does not buff itself.
- **Snapshot of "creatures you control at resolution time"**: `on_activate_ability` collects `creature_ids` by iterating `state.objects.values()` and filtering for `zone == Zone::Battlefield && controller == controller` at the moment of resolution. Entries are pushed individually to `until_end_of_turn_effects` keyed by ObjectId. Creatures entering later will not have an entry. This correctly implements the ruling: "Only creatures you control when Selfless Cathar's ability resolves will be affected."
- **Activating with no other creatures**: `SacrificeCost::SacrificeThis` in `legal_actions` (engine.rs ~line 365) only checks that the source object is on the battlefield; no other-creature requirement exists. If `creature_ids` ends up empty after the sacrifice (only the cathar was on the battlefield), the for loop does nothing. Correct per ruling: "You can activate Selfless Cathar's ability even if you control no other creatures."
- **Until-end-of-turn cleanup**: `state.until_end_of_turn_effects.clear()` is called unconditionally during `Step::Cleanup` (engine.rs ~line 3021). The +1/+1 buff correctly expires at end of turn.
- **Ability speed**: `sorcery_speed_only: false` means the ability is available any time the player has priority. The oracle text has no "activate only as a sorcery" restriction, so instant speed is correct.
- **Mana cost accuracy**: Activated ability declares `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)])` = `{1}{W}`. Matches oracle text exactly.
- **Card data accuracy**: Card cost `{W}`, subtypes `["Human", "Cleric"]`, P/T 1/1, oracle text field verbatim — all match oracle text. (Note: the doc comment at line 7 says "Human Soldier" instead of "Human Cleric" but this is a stale comment and does not affect any game behavior.)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Activating ability sacrifices the cathar and grants +1/+1 to other creatures: `mtg-engine/tests/tier8_cards.rs:20` (`selfless_cathar_pump_all_creatures`) — TESTED
- Cathar itself does not receive the +1/+1 buff (is excluded because it's in Zone::Graveyard at resolution time): NOT TESTED explicitly (test only checks the bear's stats, not cathar's effective power from graveyard)
- Ruling: "You can activate Selfless Cathar's ability even if you control no other creatures": NOT TESTED
- "Only creatures you control when the ability resolves are affected" (new creatures entering after do not get the buff): NOT TESTED
- Until-end-of-turn cleanup removes the buff at cleanup step: NOT TESTED
