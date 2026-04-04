## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Engine auto-selects the exiled creature without presenting a player choice
  - Oracle text says: `"As an additional cost to cast this spell, exile a creature card from your graveyard."`
  - Code does: `engine.rs:1574–1584` — `// Pick highest-power creatures first (better default for Corpse Lunge). … exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first` — the engine unconditionally picks the highest-power creature(s) from the graveyard. The `Action::CastSpell` struct has no field to encode which specific creature should be exiled under `ExileCreaturesFromGraveyard`, so the player is never presented with a choice. Contrast with the `SacrificeCreature` additional cost, which generates one `CastSpell` action per eligible creature, giving the player real agency.

- Test `corpse_lunge_picks_highest_power_creature` (tier8_cards.rs:538) enshrines the wrong auto-selection behavior
  - Oracle text says: `"exile a creature card from your graveyard"` (player chooses which one)
  - Code does: the test asserts `assert_eq!(big_obj.zone, Zone::Exile, "Highest-power creature should be exiled")` — it verifies and locks in the auto-selection of the highest-power creature as the only possible behavior, rather than testing that the player has been offered a choice.

### Tricky interactions checked

- **Player choice of exiled creature**: FAIL. The oracle's additional cost ("exile a creature card") requires the player to choose which creature to exile. The engine at `engine.rs:1574–1584` auto-selects the highest-power creature without presenting a choice. The `Action::CastSpell` action has no slot for a player-chosen exile target under `ExileCreaturesFromGraveyard`.
- **Exiled creature's power used for damage**: PASS. `engine.rs:1589–1592` stores the first (highest-power) exiled creature's power in `card_state["exiled_power"]`, and `corpse_lunge.rs:40–43` reads it at resolution. Encoding is `ObjectId(*power as u64)` / `id.0 as i32` — a hack that reuses ObjectId as an integer, but it round-trips correctly for reasonable power values.
- **Damage type is non-combat**: PASS. `corpse_lunge.rs:55` emits `GameEvent::NonCombatDamageDealt` — correct for an instant-speed spell.
- **`move_spell_after_resolve` called**: PASS. `corpse_lunge.rs:66` calls `state.move_spell_after_resolve(object_id)`, so the spell goes to the graveyard normally or exile if cast with flashback.
- **Target creature must be on battlefield at resolution**: PASS. `corpse_lunge.rs:51` checks `if obj.zone == Zone::Battlefield` before marking damage; if the target has left, the damage step is silently skipped (correct per MTG rule 608.2b — spell countered by rules if no legal targets at resolution).
- **Zero-power exiled creature deals no damage**: PASS. `corpse_lunge.rs:46` computes `power.max(0) as u32` and the outer check `if damage > 0` means 0-power exiles do nothing.
- **Cannot cast without graveyard creature**: PASS. `engine.rs:543–554` checks `creature_count < *n` and skips the card's cast actions if fewer than 1 creature card exists in the graveyard.
- **Mana cost {2}{B}**: PASS. `card_data` has `Generic(2), Colored(Color::Black)`.
- **Type line — Instant only**: PASS. `card_types: vec![CardType::Instant]`.
- **No triggered abilities declared**: PASS. The card has no `triggered_abilities`, consistent with oracle text.
- **Target any creature (no restriction)**: PASS. `target_requirement` returns `TargetRequirement::Creature` with no filter — matches the oracle's unqualified "target creature."

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Player chooses which creature to exile: NOT TESTED (no test verifies that the player is offered a choice; the test that exists asserts the opposite — auto-selection)
- Damage equals exiled creature's power: `tier8_cards.rs:498` (`corpse_lunge_deals_damage_equal_to_exiled_power`)
- Highest-power creature auto-selected when multiple available: `tier8_cards.rs:538` (`corpse_lunge_picks_highest_power_creature`) — NOTE: this test enshrines the wrong behavior
- No graveyard creature: `tier8_cards.rs:524` (`corpse_lunge_no_graveyard_creature_deals_no_damage`) — tests forced-cast edge case (bypasses legal-action check via `submit_action` directly), useful sanity test but does not test that the spell is uncollectable without a graveyard creature
- Spell goes to graveyard after resolution: NOT TESTED
- Damage type is non-combat: NOT TESTED
- Target leaves battlefield before resolution (fizzle): NOT TESTED
- Zero-power exiled creature: NOT TESTED
- Ruling [2013-04-15] "exactly one creature card — not zero, not more": partial (`tier8_cards.rs:524` tests zero-creature-in-graveyard edge case via forced cast; rule 543–553 enforces it in legal actions, but this enforcement is not directly tested)
