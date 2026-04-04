## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.
Equip {2}
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Land card counted in bonus (ruling: "The land card is counted when calculating the bonus")**: PASS. In `on_attacks`, `cards_milled += 1` is incremented unconditionally before `if is_land { break; }`, so the land card is counted. The land is also moved to the graveyard before the break.

- **All revealed cards put into graveyard (including land)**: PASS. `state.move_object(card_id, Zone::Graveyard)` runs for every card, including the terminal land, before the break.

- **Defending player lookup from combat state**: PASS. `state.combat.as_ref().and_then(|c| c.attackers.get(&creature_id).copied())` correctly retrieves the defending `PlayerId` for the attacker, which `declare_attackers` (combat.rs:26) sets at attacker-declaration time. Attacking a planeswalker is handled by the same mechanism since `declare_attackers` always stores a `PlayerId` (the planeswalker's controller) for the attacker.

- **Trigger fires for equipment (not for the creature)**: PASS. The `AttackersDeclared` handler in `triggers.rs:699–720` explicitly iterates objects with `attached_to == Some(attacker_id)` and collects `AttacksTrigger` entries for them. The `object_id` stored in the trigger is the equipment's ID, so `on_attacks` is called with `self_id = equipment_id`. The equipment then reads its own `attached_to` to find the creature.

- **Empty library (no cards at all)**: PASS. `if player.library_order.is_empty() { break; }` at the top of the loop exits immediately with `cards_milled == 0`; no bonus is applied and no graveyard moves happen.

- **Library with no lands (mills entire library)**: PASS. The loop runs until `library_order` is empty, moving every card to the graveyard. The creature gets `+cards_milled/+0`.

- **Until-end-of-turn effect cleanup**: PASS. `state.until_end_of_turn_effects.clear()` is called in the `Step::Cleanup` handler in `engine.rs:3021`, removing the power bonus at end of turn.

- **Trigger fizzle if equipment destroyed before resolution**: PASS (not an issue in current engine). `triggers::process_triggers` resolves all triggers synchronously before returning control to the game loop, so players cannot cast instants between trigger collection and resolution. The `zone == Battlefield` guard in `resolve_next_trigger` never blocks the normal case.

- **Equip ability restricted to sorcery speed**: PASS. `ActivatedAbilityDef { sorcery_speed_only: true, .. }` is set; the engine enforces this in legal-actions generation.

- **Equip targets only creatures the controller controls**: PASS. `target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl))` limits equip targets to the controller's own creatures.

- **Library card types correctly read in actual gameplay**: PASS. `setup_game` (engine.rs:2681) sets `obj.card_types = card_data.card_types.clone()` on every library object, so `state.get_object(card_id).map(|o| o.card_types.contains(&CardType::Land))` works correctly in real games.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:

- Land card counted in bonus (ruling): `mtg-engine/tests/tier9_cards.rs:300` (`trepanation_blade_attack_trigger_mills_and_pumps` — 3 cards milled including land, +3/+0 applied)
- All cards including land put to graveyard: `mtg-engine/tests/tier9_cards.rs:333` (all 3 cards asserted in Zone::Graveyard)
- Stops at first land when land is first: `mtg-engine/tests/tier9_cards.rs:348` (`trepanation_blade_stops_at_first_land` — only 1 card milled, +1/+0)
- Card data (cost, types, subtypes): `mtg-engine/tests/tier9_cards.rs:290` (`trepanation_blade_card_data`)
- Empty library: NOT TESTED
- Library with no lands (mills all): NOT TESTED
- Attacking a planeswalker (defending player is planeswalker's controller): NOT TESTED
- Until-end-of-turn effect cleanup at end of turn: NOT TESTED
